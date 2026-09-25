//! The transfer engine: a queue of jobs drained by worker tasks.
//!
//! Jobs own one file transfer end-to-end. The engine dials its *own*
//! connection per attempt (via [`Connector`]), so browsing never blocks on a
//! busy transfer — the same model Transmit uses. Interrupted transfers resume
//! from byte offsets: the engine derives them from the local (download) or
//! remote (upload) file size, and `RemoteFs::open_read/open_write` continues
//! from there.
//!
//! Control flow per job:
//! `Queued → Active → Completed | Failed | Paused | Canceled`
//! — pausing mid-file simply aborts the attempt; resuming re-dials and seeks.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use flow_core::{
    ConnectionId, Credentials, CoreError, CoreResult, FilePath, RemoteFs, TransferDirection,
    TransferId, TransferStatus,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex, Notify};

use crate::progress::{RateLimiter, RetryPolicy, SpeedEstimator};
use crate::record::{TransferEvent, TransferRecord};

/// Size of one copy-loop chunk (also the control-check granularity).
const CHUNK_SIZE: usize = 64 * 1024;
/// Minimum gap between progress events, so the UI never floods.
const EMIT_INTERVAL: Duration = Duration::from_millis(200);

/// Supplies a fresh adapter connection for one transfer attempt.
///
/// The engine dials per attempt so browsing stays independent of transfers.
#[async_trait]
pub trait Connector: Send + Sync {
    async fn connect(&self, creds: &Credentials) -> CoreResult<Box<dyn RemoteFs>>;
}

/// What to move, and between where.
#[derive(Debug, Clone)]
pub struct TransferSpec {
    pub connection_id: ConnectionId,
    pub connection_name: String,
    pub direction: TransferDirection,
    pub remote_path: FilePath,
    pub local_path: FilePath,
    /// Display name (usually the file's basename).
    pub file_name: String,
}

/// Cooperative pause/cancel flag shared with the copy loop.
#[derive(Debug, Clone, Default)]
pub struct ControlFlags(Arc<std::sync::atomic::AtomicU8>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlState {
    Running,
    Pausing,
    Canceled,
}

impl ControlFlags {
    fn state(&self) -> ControlState {
        match self.0.load(Ordering::Relaxed) {
            1 => ControlState::Pausing,
            2 => ControlState::Canceled,
            _ => ControlState::Running,
        }
    }

    fn request_pause(&self) {
        self.0.store(1, Ordering::Relaxed);
    }

    fn request_cancel(&self) {
        self.0.store(2, Ordering::Relaxed);
    }
}

/// Everything tracked for one job, across attempts.
struct Job {
    id: TransferId,
    spec: TransferSpec,
    creds: Credentials,
    status: TransferStatus,
    transferred: u64,
    total: u64,
    speed: u64,
    error: Option<String>,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    attempts: u32,
    control: ControlFlags,
}

impl Job {
    fn record(&self) -> TransferRecord {
        let mut record = TransferRecord {
            id: self.id.clone(),
            file_name: self.spec.file_name.clone(),
            direction: self.spec.direction,
            status: self.status,
            size: self.total,
            transferred: self.transferred,
            speed: self.speed,
            eta_secs: None,
            connection_id: self.spec.connection_id.as_str().to_string(),
            connection_name: self.spec.connection_name.clone(),
            remote_path: self.spec.remote_path.as_str().to_string(),
            local_path: self.spec.local_path.as_str().to_string(),
            started_at: self.started_at,
            finished_at: self.finished_at,
            error: self.error.clone(),
            resumable: true,
        };
        record.eta_secs = record.compute_eta();
        record
    }
}

/// Engine-visible outcome of one attempt.
enum AttemptOutcome {
    Done,
    Paused,
    Canceled,
    Failed(CoreError),
}

/// Queue + workers + progress + retry.
pub struct TransferEngine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    state: Mutex<EngineState>,
    events: mpsc::UnboundedSender<TransferEvent>,
    connector: Arc<dyn Connector>,
    retry: RetryPolicy,
    rate_limit_bps: Option<u64>,
    notify: Notify,
}

#[derive(Default)]
struct EngineState {
    jobs: HashMap<TransferId, Job>,
    pending: VecDeque<TransferId>,
}

impl TransferEngine {
    /// Create an engine. Call [`TransferEngine::run_worker`] once per
    /// concurrent slot (typically 2–3).
    pub fn new(
        connector: Arc<dyn Connector>,
        events: mpsc::UnboundedSender<TransferEvent>,
        retry: RetryPolicy,
        rate_limit_bps: Option<u64>,
    ) -> Self {
        Self {
            inner: Arc::new(EngineInner {
                state: Mutex::new(EngineState::default()),
                events,
                connector,
                retry,
                rate_limit_bps,
                notify: Notify::new(),
            }),
        }
    }

    /// Enqueue one file transfer under the given id.
    pub async fn enqueue(&self, id: TransferId, spec: TransferSpec, creds: Credentials) {
        let job = Job {
            id: id.clone(),
            status: TransferStatus::Queued,
            transferred: 0,
            total: 0,
            speed: 0,
            error: None,
            started_at: None,
            finished_at: None,
            attempts: 0,
            control: ControlFlags::default(),
            spec,
            creds,
        };
        let record = job.record();
        let mut state = self.inner.state.lock().await;
        state.jobs.insert(id.clone(), job);
        state.pending.push_back(id);
        drop(state);
        let _ = self.inner.events.send(TransferEvent::Update(record));
        self.inner.notify.notify_one();
    }

    /// Pause a transfer. Running transfers stop at the next chunk boundary;
    /// queued transfers leave the queue. Resuming re-dials from the offset.
    pub async fn pause(&self, id: &TransferId) -> CoreResult<()> {
        let record = {
            let mut state = self.inner.state.lock().await;
            let EngineState { jobs, pending } = &mut *state;
            let job = jobs
                .get_mut(id)
                .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
            match job.status {
                TransferStatus::Queued => {
                    pending.retain(|pending_id| pending_id != id);
                    job.status = TransferStatus::Paused;
                    Some(job.record())
                }
                TransferStatus::Active => {
                    job.control.request_pause();
                    None
                }
                _ => None,
            }
        };
        if let Some(record) = record {
            let _ = self.inner.events.send(TransferEvent::Update(record));
        }
        Ok(())
    }

    /// Resume a paused transfer from its byte offset.
    pub async fn resume(&self, id: &TransferId) -> CoreResult<()> {
        let record = {
            let mut state = self.inner.state.lock().await;
            let EngineState { jobs, pending } = &mut *state;
            let job = jobs
                .get_mut(id)
                .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
            if job.status != TransferStatus::Paused {
                return Ok(());
            }
            job.status = TransferStatus::Queued;
            job.error = None;
            pending.push_back(id.clone());
            Some(job.record())
        };
        if let Some(record) = record {
            let _ = self.inner.events.send(TransferEvent::Update(record));
        }
        self.inner.notify.notify_one();
        Ok(())
    }

    /// Cancel a transfer. Terminal records stay until [`Self::clear_finished`].
    pub async fn cancel(&self, id: &TransferId) -> CoreResult<()> {
        let record = {
            let mut state = self.inner.state.lock().await;
            let EngineState { jobs, pending } = &mut *state;
            let job = jobs
                .get_mut(id)
                .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
            match job.status {
                TransferStatus::Queued | TransferStatus::Paused => {
                    // Not running anywhere — finalize immediately.
                    pending.retain(|pending_id| pending_id != id);
                    job.status = TransferStatus::Canceled;
                    job.finished_at = Some(now_millis());
                    Some(job.record())
                }
                TransferStatus::Active => {
                    job.control.request_cancel();
                    None
                }
                _ => None,
            }
        };
        if let Some(record) = record {
            let _ = self.inner.events.send(TransferEvent::Update(record));
        }
        Ok(())
    }

    /// Snapshots of all transfers, most recently started first.
    pub async fn snapshots(&self) -> Vec<TransferRecord> {
        let state = self.inner.state.lock().await;
        let mut records: Vec<TransferRecord> = state.jobs.values().map(|job| job.record()).collect();
        records.sort_by_key(|record| std::cmp::Reverse(record.started_at.unwrap_or(0)));
        records
    }

    /// Drop completed/failed/canceled records from the list.
    pub async fn clear_finished(&self) {
        let mut state = self.inner.state.lock().await;
        let finished: Vec<TransferId> = state
            .jobs
            .iter()
            .filter(|(_, job)| {
                matches!(
                    job.status,
                    TransferStatus::Completed | TransferStatus::Failed | TransferStatus::Canceled
                )
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in finished {
            state.jobs.remove(&id);
            state.pending.retain(|pending_id| pending_id != &id);
        }
    }

    /// Worker loop: take the next pending job and run it to a terminal state.
    /// Spawn one task per desired concurrent slot.
    pub async fn run_worker(self: Arc<Self>) {
        loop {
            let next = {
                let mut state = self.inner.state.lock().await;
                state.pending.pop_front()
            };
            match next {
                Some(id) => self.clone().run_job(id).await,
                None => self.inner.notify.notified().await,
            }
        }
    }

    /// Run one job through retries to a terminal state.
    async fn run_job(self: Arc<Self>, id: TransferId) {
        loop {
            let (control, spec, creds, attempts) = {
                let mut state = self.inner.state.lock().await;
                let Some(job) = state.jobs.get_mut(&id) else {
                    return; // cleared while queued
                };
                job.status = TransferStatus::Active;
                if job.started_at.is_none() {
                    job.started_at = Some(now_millis());
                }
                job.attempts += 1;
                let snapshot = (
                    job.control.clone(),
                    job.spec.clone(),
                    job.creds.clone(),
                    job.attempts,
                );
                let record = job.record();
                drop(state);
                let _ = self.inner.events.send(TransferEvent::Update(record));
                snapshot
            };

            match self
                .attempt_transfer(&id, &spec, &creds, &control)
                .await
            {
                AttemptOutcome::Done => {
                    self.finalize(&id, TransferStatus::Completed, None).await;
                    return;
                }
                AttemptOutcome::Paused => {
                    self.finalize(&id, TransferStatus::Paused, None).await;
                    return;
                }
                AttemptOutcome::Canceled => {
                    self.finalize(&id, TransferStatus::Canceled, None).await;
                    return;
                }
                AttemptOutcome::Failed(e) => {
                    if is_retryable(&e) && self.inner.retry.should_retry(attempts) {
                        tokio::time::sleep(self.inner.retry.delay_for(attempts)).await;
                        self.set_status(&id, TransferStatus::Queued, None).await;
                        continue;
                    }
                    self.finalize(&id, TransferStatus::Failed, Some(e.to_string()))
                        .await;
                    return;
                }
            }
        }
    }

    /// One dial → measure → stream → copy pass.
    async fn attempt_transfer(
        &self,
        id: &TransferId,
        spec: &TransferSpec,
        creds: &Credentials,
        control: &ControlFlags,
    ) -> AttemptOutcome {
        let mut fs = match self.inner.connector.connect(creds).await {
            Ok(fs) => fs,
            Err(e) => return AttemptOutcome::Failed(e),
        };

        // Total size and resume offset come from the file systems themselves.
        let (total, offset) = match self.measure(&mut *fs, spec).await {
            Ok(pair) => pair,
            Err(e) => {
                let _ = fs.disconnect().await;
                return AttemptOutcome::Failed(e);
            }
        };
        {
            let mut state = self.inner.state.lock().await;
            if let Some(job) = state.jobs.get_mut(id) {
                job.total = total;
                job.transferred = offset;
            }
        }

        let outcome = match spec.direction {
            TransferDirection::Download => {
                self.stream_download(&mut *fs, id, spec, offset, control)
                    .await
            }
            TransferDirection::Upload => self.stream_upload(&mut *fs, id, spec, offset, control).await,
        };

        // The connection is single-purpose; always hang up cleanly.
        let _ = fs.disconnect().await;
        outcome
    }

    /// Total size and resume offset for the spec's direction.
    async fn measure(
        &self,
        fs: &mut dyn RemoteFs,
        spec: &TransferSpec,
    ) -> CoreResult<(u64, u64)> {
        match spec.direction {
            TransferDirection::Download => {
                let remote = fs.stat(&spec.remote_path).await?;
                let local_offset = tokio::fs::metadata(spec.local_path.as_str())
                    .await
                    .map(|meta| meta.len())
                    .unwrap_or(0);
                Ok((remote.size, local_offset.min(remote.size)))
            }
            TransferDirection::Upload => {
                let local_size = tokio::fs::metadata(spec.local_path.as_str())
                    .await
                    .map_err(|e| CoreError::Io(e.to_string()))?
                    .len();
                let remote_offset = match fs.stat(&spec.remote_path).await {
                    Ok(remote) => remote.size,
                    // Missing remote file simply means a fresh upload.
                    Err(CoreError::NotFound(_)) => 0,
                    Err(e) => return Err(e),
                };
                Ok((local_size, remote_offset.min(local_size)))
            }
        }
    }

    /// Copy remote → local with control checks, progress sampling, and
    /// bandwidth limiting.
    async fn stream_download(
        &self,
        fs: &mut dyn RemoteFs,
        id: &TransferId,
        spec: &TransferSpec,
        offset: u64,
        control: &ControlFlags,
    ) -> AttemptOutcome {
        let mut reader = match fs.open_read(&spec.remote_path, offset).await {
            Ok(reader) => reader,
            Err(e) => return AttemptOutcome::Failed(e),
        };
        let mut file = match open_local_for_download(&spec.local_path, offset).await {
            Ok(file) => file,
            Err(e) => return AttemptOutcome::Failed(e),
        };
        self.copy_loop(&mut reader, &mut file, id, offset, control)
            .await
    }

    /// Copy local → remote with control checks, progress sampling, and
    /// bandwidth limiting.
    async fn stream_upload(
        &self,
        fs: &mut dyn RemoteFs,
        id: &TransferId,
        spec: &TransferSpec,
        offset: u64,
        control: &ControlFlags,
    ) -> AttemptOutcome {
        let mut writer = match fs.open_write(&spec.remote_path, offset).await {
            Ok(writer) => writer,
            Err(e) => return AttemptOutcome::Failed(e),
        };
        let mut file = match tokio::fs::File::open(spec.local_path.as_str()).await {
            Ok(file) => file,
            Err(e) => return AttemptOutcome::Failed(CoreError::Io(e.to_string())),
        };
        if offset > 0 {
            // Continue reading the local file where the remote left off.
            if let Err(e) = file.seek(std::io::SeekFrom::Start(offset)).await {
                return AttemptOutcome::Failed(CoreError::Io(e.to_string()));
            }
        }
        self.copy_loop(&mut file, &mut writer, id, offset, control)
            .await
    }

    /// The engine's core loop: chunked copy with cooperative control.
    async fn copy_loop(
        &self,
        reader: &mut (dyn AsyncRead + Send + Unpin),
        writer: &mut (dyn AsyncWrite + Send + Unpin),
        id: &TransferId,
        start_offset: u64,
        control: &ControlFlags,
    ) -> AttemptOutcome {
        let mut buf = vec![0u8; CHUNK_SIZE];
        let started = Instant::now();
        let mut estimator = SpeedEstimator::new();
        let mut limiter = RateLimiter::new(self.inner.rate_limit_bps);
        let mut last_emit = Instant::now() - EMIT_INTERVAL;
        let mut transferred = start_offset;

        loop {
            match control.state() {
                ControlState::Pausing => return AttemptOutcome::Paused,
                ControlState::Canceled => return AttemptOutcome::Canceled,
                ControlState::Running => {}
            }

            let read = match reader.read(&mut buf).await {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => return AttemptOutcome::Failed(CoreError::Io(e.to_string())),
            };

            if let Some(wait) = limiter.throttle(started.elapsed(), read as u64) {
                tokio::time::sleep(wait).await;
            }
            if let Err(e) = writer.write_all(&buf[..read]).await {
                return AttemptOutcome::Failed(CoreError::Io(e.to_string()));
            }
            transferred += read as u64;

            if last_emit.elapsed() >= EMIT_INTERVAL {
                let speed = estimator.sample(started.elapsed(), transferred);
                let record = self.progress_record(id, transferred, speed).await;
                last_emit = Instant::now();
                let _ = self.inner.events.send(TransferEvent::Update(record));
            }
        }

        if let Err(e) = writer.flush().await {
            return AttemptOutcome::Failed(CoreError::Io(e.to_string()));
        }
        // Final sample so the completed record shows real totals.
        let speed = estimator.sample(started.elapsed(), transferred);
        let record = self.progress_record(id, transferred, speed).await;
        let _ = self.inner.events.send(TransferEvent::Update(record));
        AttemptOutcome::Done
    }

    /// Update in-flight numbers on the job and snapshot it.
    async fn progress_record(
        &self,
        id: &TransferId,
        transferred: u64,
        speed: u64,
    ) -> TransferRecord {
        let mut state = self.inner.state.lock().await;
        let job = state
            .jobs
            .get_mut(id)
            .expect("in-flight job must exist in the engine");
        job.transferred = transferred;
        job.speed = speed;
        job.total = job.total.max(transferred);
        job.record()
    }

    /// Set an interim status (used between retry attempts).
    async fn set_status(&self, id: &TransferId, status: TransferStatus, error: Option<String>) {
        let mut state = self.inner.state.lock().await;
        if let Some(job) = state.jobs.get_mut(id) {
            job.status = status;
            job.error = error;
        }
    }

    /// Finalize a job with a terminal state and emit the snapshot.
    async fn finalize(&self, id: &TransferId, status: TransferStatus, error: Option<String>) {
        let record = {
            let mut state = self.inner.state.lock().await;
            let Some(job) = state.jobs.get_mut(id) else {
                return;
            };
            job.status = status;
            job.error = error;
            if matches!(
                status,
                TransferStatus::Completed
                    | TransferStatus::Failed
                    | TransferStatus::Canceled
                    | TransferStatus::Paused
            ) {
                job.finished_at = Some(now_millis());
            }
            job.record()
        };
        let _ = self.inner.events.send(TransferEvent::Update(record));
    }
}

/// Local file handle for a download: append when resuming, truncate when fresh.
async fn open_local_for_download(
    path: &FilePath,
    offset: u64,
) -> CoreResult<tokio::fs::File> {
    if offset > 0 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(path.as_str())
            .await
            .map_err(|e| CoreError::Io(e.to_string()))
    } else {
        tokio::fs::File::create(path.as_str())
            .await
            .map_err(|e| CoreError::Io(e.to_string()))
    }
}

/// Whether retrying can plausibly help. Auth/missing-file/path errors are
/// deterministic — retrying just burns the user's time.
fn is_retryable(e: &CoreError) -> bool {
    !matches!(
        e,
        CoreError::AuthFailed { .. }
            | CoreError::NotFound(_)
            | CoreError::Permission(_)
            | CoreError::InvalidPath(_)
    )
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
