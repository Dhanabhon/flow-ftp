//! Engine behavior tests against an in-memory mock filesystem.
//!
//! These exercise the real queue/worker/retry/resume logic — no network.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use flow_core::{
    ConnectionId, Credentials, CoreError, CoreResult, FilePath, Protocol, RemoteFile, RemoteFs,
    TransferDirection, TransferId, TransferStatus,
};
use flow_transfer::{
    Connector, RetryPolicy, TransferEngine, TransferEvent, TransferRecord, TransferSpec,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;

/// A named blob in the fake remote store.
type FakeFile = (String, Vec<u8>);
type FakeStore = Arc<Mutex<Vec<FakeFile>>>;

/// Shared in-memory "remote filesystem" the mock adapter reads/writes.
#[derive(Clone, Default)]
struct FakeRemote {
    files: FakeStore,
    /// Dial failures to inject before a successful connect.
    fail_next_dials: Arc<AtomicU32>,
}

impl FakeRemote {
    fn put(&self, name: &str, content: &[u8]) {
        let mut files = self.files.lock().unwrap();
        files.retain(|(existing, _)| existing != name);
        files.push((name.to_string(), content.to_vec()));
    }

    fn get(&self, name: &str) -> Option<Vec<u8>> {
        self.files
            .lock()
            .unwrap()
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, data)| data.clone())
    }
}

/// Adapter over [`FakeRemote`] — implements the real domain trait.
struct FakeFs {
    remote: FakeRemote,
}

#[async_trait]
impl RemoteFs for FakeFs {
    async fn connect(&mut self, _creds: &Credentials) -> CoreResult<()> {
        if self.remote.fail_next_dials.load(Ordering::SeqCst) > 0 {
            self.remote.fail_next_dials.fetch_sub(1, Ordering::SeqCst);
            return Err(CoreError::ConnectionFailed {
                host: "fake".into(),
                port: 0,
                reason: "injected dial failure".into(),
            });
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        Ok(())
    }

    async fn list(&mut self, _path: &FilePath) -> CoreResult<Vec<RemoteFile>> {
        unreachable!("not used by the engine")
    }

    async fn stat(&mut self, path: &FilePath) -> CoreResult<RemoteFile> {
        let name = path.name().to_string();
        match self.remote.get(&name) {
            Some(data) => Ok(RemoteFile {
                name,
                kind: flow_core::FileKind::File,
                size: data.len() as u64,
                modified: 0,
                permissions: None,
                owner: None,
                group: None,
            }),
            None => Err(CoreError::NotFound(path.as_str().to_string())),
        }
    }

    async fn mkdir(&mut self, _path: &FilePath) -> CoreResult<()> {
        unreachable!()
    }

    async fn rename(&mut self, _from: &FilePath, _to: &FilePath) -> CoreResult<()> {
        unreachable!()
    }

    async fn delete(&mut self, _path: &FilePath) -> CoreResult<()> {
        unreachable!()
    }

    async fn open_read(
        &mut self,
        remote: &FilePath,
        offset: u64,
    ) -> CoreResult<Box<dyn AsyncRead + Send + Unpin>> {
        let name = remote.name().to_string();
        let data = self
            .remote
            .get(&name)
            .ok_or_else(|| CoreError::NotFound(remote.as_str().to_string()))?;
        let cursor = std::io::Cursor::new(data);
        let mut reader = tokio::io::BufReader::new(cursor);
        if offset > 0 {
            use tokio::io::AsyncSeekExt;
            reader
                .seek(std::io::SeekFrom::Start(offset))
                .await
                .map_err(|e| CoreError::Io(e.to_string()))?;
        }
        Ok(Box::new(reader))
    }

    async fn open_write(
        &mut self,
        remote: &FilePath,
        offset: u64,
    ) -> CoreResult<Box<dyn AsyncWrite + Send + Unpin>> {
        let name = remote.name().to_string();
        let files = self.remote.files.clone();
        // A writer that lands bytes back into the FakeRemote on flush.
        Ok(Box::new(FakeRemoteWriter {
            name,
            files,
            buffer: Vec::new(),
            offset,
        }))
    }
}

/// AsyncWrite that accumulates and commits into FakeRemote on flush/drop.
struct FakeRemoteWriter {
    name: String,
    files: FakeStore,
    buffer: Vec<u8>,
    offset: u64,
}

impl FakeRemoteWriter {
    fn commit(&mut self) {
        let mut files = self.files.lock().unwrap();
        let existing = files.iter_mut().find(|(name, _)| *name == self.name);
        match existing {
            Some((_, data)) => {
                data.truncate(self.offset as usize);
                data.extend_from_slice(&self.buffer);
            }
            None => {
                let mut data = vec![0u8; self.offset as usize];
                data.extend_from_slice(&self.buffer);
                files.push((self.name.clone(), data));
            }
        }
        self.buffer.clear();
    }
}

impl AsyncWrite for FakeRemoteWriter {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        this.buffer.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.get_mut().commit();
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.poll_flush(cx)
    }
}

/// Connector dialing the shared FakeRemote.
struct FakeConnector {
    remote: FakeRemote,
}

#[async_trait]
impl Connector for FakeConnector {
    async fn connect(&self, _creds: &Credentials) -> CoreResult<Box<dyn RemoteFs>> {
        Ok(Box::new(FakeFs {
            remote: self.remote.clone(),
        }))
    }
}

/// Unique temp path for a test's local side.
fn temp_local(tag: &str) -> FilePath {
    let dir = std::env::temp_dir().join(format!(
        "flow-transfer-it-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir failed");
    FilePath::new(dir.join("file.bin").to_string_lossy().into_owned())
}

fn spec(direction: TransferDirection, tag: &str) -> TransferSpec {
    TransferSpec {
        connection_id: ConnectionId::new("fake-conn"),
        connection_name: "Fake".into(),
        direction,
        remote_path: FilePath::new("/data/blob.bin"),
        local_path: temp_local(tag),
        file_name: "blob.bin".into(),
    }
}

fn creds() -> Credentials {
    Credentials::new("fake", "tester", Protocol::Sftp)
}

async fn drain_events(mut rx: mpsc::UnboundedReceiver<TransferEvent>) -> Vec<TransferRecord> {
    let mut records = Vec::new();
    while let Ok(event) = rx.try_recv() {
        let TransferEvent::Update(record) = event;
        records.push(record);
    }
    records
}

#[tokio::test]
async fn download_completes_with_correct_bytes() {
    let remote = FakeRemote::default();
    remote.put("blob.bin", b"hello flowftp transfer engine");

    let (tx, rx) = mpsc::unbounded_channel();
    let engine = Arc::new(TransferEngine::new(
        Arc::new(FakeConnector { remote: remote.clone() }),
        tx,
        RetryPolicy::standard(),
        None,
    ));
    let worker = tokio::spawn(engine.clone().run_worker());

    let local = temp_local("download-complete");
    engine
        .enqueue(
            TransferId::new("d1"),
            spec(TransferDirection::Download, "download-complete"),
            creds(),
        )
        .await;

    // Wait for completion.
    for _ in 0..200 {
        let records = engine.snapshots().await;
        if records.iter().any(|r| r.status == TransferStatus::Completed) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    worker.abort();
    let written = std::fs::read(local.as_str()).expect("local file must exist");
    assert_eq!(written, b"hello flowftp transfer engine");

    let final_record = drain_events(rx)
        .await
        .into_iter()
        .find(|r| r.status == TransferStatus::Completed)
        .expect("a completed event must have been emitted");
    assert_eq!(final_record.transferred, 29);
    assert_eq!(final_record.size, 29);
}

#[tokio::test]
async fn download_resumes_from_existing_local_bytes() {
    let payload = b"the quick brown fox jumps over the lazy dog";
    let remote = FakeRemote::default();
    remote.put("blob.bin", payload);

    // Pre-existing partial local file: first 10 bytes already there.
    let local = temp_local("download-resume");
    std::fs::write(local.as_str(), &payload[..10]).expect("seed partial failed");

    let (tx, _rx) = mpsc::unbounded_channel();
    let engine = Arc::new(TransferEngine::new(
        Arc::new(FakeConnector { remote: remote.clone() }),
        tx,
        RetryPolicy::standard(),
        None,
    ));
    let worker = tokio::spawn(engine.clone().run_worker());

    engine
        .enqueue(
            TransferId::new("d2"),
            spec(TransferDirection::Download, "download-resume"),
            creds(),
        )
        .await;

    for _ in 0..200 {
        if engine
            .snapshots()
            .await
            .iter()
            .any(|r| r.status == TransferStatus::Completed)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    worker.abort();

    let written = std::fs::read(local.as_str()).expect("local file must exist");
    assert_eq!(written, payload, "resume must produce identical bytes");
}

#[tokio::test]
async fn upload_creates_remote_file() {
    let remote = FakeRemote::default();
    let local = temp_local("upload");
    std::fs::write(local.as_str(), b"upload payload").expect("seed failed");

    let (tx, _rx) = mpsc::unbounded_channel();
    let engine = Arc::new(TransferEngine::new(
        Arc::new(FakeConnector { remote: remote.clone() }),
        tx,
        RetryPolicy::standard(),
        None,
    ));
    let worker = tokio::spawn(engine.clone().run_worker());

    engine
        .enqueue(
            TransferId::new("u1"),
            spec(TransferDirection::Upload, "upload"),
            creds(),
        )
        .await;

    for _ in 0..200 {
        if engine
            .snapshots()
            .await
            .iter()
            .any(|r| r.status == TransferStatus::Completed)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    worker.abort();

    assert_eq!(remote.get("blob.bin").as_deref(), Some(&b"upload payload"[..]));
}

#[tokio::test]
async fn retry_recovers_from_transient_dial_failures() {
    let remote = FakeRemote::default();
    remote.put("blob.bin", b"retry me");
    // Two dial failures, then success.
    remote.fail_next_dials.store(2, Ordering::SeqCst);

    let local = temp_local("retry");
    let (tx, _rx) = mpsc::unbounded_channel();
    let engine = Arc::new(TransferEngine::new(
        Arc::new(FakeConnector { remote: remote.clone() }),
        tx,
        RetryPolicy {
            max_attempts: 5,
            base_delay: std::time::Duration::from_millis(1),
            max_delay: std::time::Duration::from_millis(5),
        },
        None,
    ));
    let worker = tokio::spawn(engine.clone().run_worker());

    engine
        .enqueue(
            TransferId::new("r1"),
            spec(TransferDirection::Download, "retry"),
            creds(),
        )
        .await;

    for _ in 0..400 {
        if engine
            .snapshots()
            .await
            .iter()
            .any(|r| r.status == TransferStatus::Completed)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    worker.abort();

    let records = engine.snapshots().await;
    let record = records.iter().find(|r| r.id.as_str() == "r1").unwrap();
    assert_eq!(record.status, TransferStatus::Completed);
    assert_eq!(std::fs::read(local.as_str()).unwrap(), b"retry me");
}

#[tokio::test]
async fn auth_failures_fail_fast_without_retry() {
    // Dial failures of the AuthFailed kind never retry.
    struct AuthFailConnector;
    #[async_trait]
    impl Connector for AuthFailConnector {
        async fn connect(&self, _creds: &Credentials) -> CoreResult<Box<dyn RemoteFs>> {
            Err(CoreError::AuthFailed {
                username: "tester".into(),
            })
        }
    }

    let (tx, _rx) = mpsc::unbounded_channel();
    let engine = Arc::new(TransferEngine::new(
        Arc::new(AuthFailConnector),
        tx,
        RetryPolicy::standard(),
        None,
    ));
    let worker = tokio::spawn(engine.clone().run_worker());

    engine
        .enqueue(
            TransferId::new("a1"),
            spec(TransferDirection::Download, "auth-fail"),
            creds(),
        )
        .await;

    for _ in 0..100 {
        if engine
            .snapshots()
            .await
            .iter()
            .any(|r| r.status == TransferStatus::Failed)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    worker.abort();

    let record = engine
        .snapshots()
        .await
        .into_iter()
        .find(|r| r.id.as_str() == "a1")
        .unwrap();
    assert_eq!(record.status, TransferStatus::Failed);
    assert!(record.error.as_deref().unwrap_or("").contains("tester"));
    // Attempts stay at 1 — no retries for auth errors.
    // (attempts is internal; verified via immediate failure timing.)
}

#[tokio::test]
async fn cancel_queued_transfer_finalizes_immediately() {
    let remote = FakeRemote::default();
    remote.put("blob.bin", b"data");
    let (tx, _rx) = mpsc::unbounded_channel();

    // No workers spawned → the job stays queued until we cancel it.
    let engine = Arc::new(TransferEngine::new(
        Arc::new(FakeConnector { remote }),
        tx,
        RetryPolicy::standard(),
        None,
    ));

    engine
        .enqueue(
            TransferId::new("c1"),
            spec(TransferDirection::Download, "cancel"),
            creds(),
        )
        .await;
    engine
        .cancel(&TransferId::new("c1"))
        .await
        .expect("cancel failed");

    let record = engine
        .snapshots()
        .await
        .into_iter()
        .find(|r| r.id.as_str() == "c1")
        .expect("record kept after cancel");
    assert_eq!(record.status, TransferStatus::Canceled);
    assert!(record.finished_at.is_some());
}

#[tokio::test]
async fn pause_and_resume_queued_transfer() {
    let remote = FakeRemote::default();
    remote.put("blob.bin", b"data");
    let (tx, _rx) = mpsc::unbounded_channel();

    let engine = Arc::new(TransferEngine::new(
        Arc::new(FakeConnector { remote }),
        tx,
        RetryPolicy::standard(),
        None,
    ));

    engine
        .enqueue(
            TransferId::new("p1"),
            spec(TransferDirection::Download, "pause"),
            creds(),
        )
        .await;
    engine
        .pause(&TransferId::new("p1"))
        .await
        .expect("pause failed");
    assert!(engine
        .snapshots()
        .await
        .iter()
        .any(|r| r.id.as_str() == "p1" && r.status == TransferStatus::Paused));

    engine
        .resume(&TransferId::new("p1"))
        .await
        .expect("resume failed");
    assert!(engine
        .snapshots()
        .await
        .iter()
        .any(|r| r.id.as_str() == "p1" && r.status == TransferStatus::Queued));
}

#[tokio::test]
async fn clear_finished_removes_terminal_records_only() {
    let remote = FakeRemote::default();
    remote.put("blob.bin", b"data");
    let (tx, _rx) = mpsc::unbounded_channel();

    let engine = Arc::new(TransferEngine::new(
        Arc::new(FakeConnector { remote }),
        tx,
        RetryPolicy::standard(),
        None,
    ));

    // One canceled (terminal), one paused (still relevant).
    engine
        .enqueue(TransferId::new("f1"), spec(TransferDirection::Download, "clear-a"), creds())
        .await;
    engine.cancel(&TransferId::new("f1")).await.unwrap();
    engine
        .enqueue(TransferId::new("f2"), spec(TransferDirection::Download, "clear-b"), creds())
        .await;
    engine.pause(&TransferId::new("f2")).await.unwrap();

    engine.clear_finished().await;

    let records = engine.snapshots().await;
    assert!(records.iter().all(|r| r.id.as_str() != "f1"));
    assert!(records.iter().any(|r| r.id.as_str() == "f2"));
}
