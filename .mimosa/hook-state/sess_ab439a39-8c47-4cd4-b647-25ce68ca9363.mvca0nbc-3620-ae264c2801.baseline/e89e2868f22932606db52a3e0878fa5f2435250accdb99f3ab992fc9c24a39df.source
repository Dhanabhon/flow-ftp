//! # flow-sync
//!
//! Folder compare for FlowFTP's Smart Sync: two directory snapshots in, one
//! sync plan out.
//!
//! The compare is a **pure function** over listings — no I/O — so it is fully
//! unit-testable and the application layer owns walking both trees. A
//! relative path present on both sides with identical size and mtime (within
//! tolerance) is in sync; anything else becomes a [`flow_core::SyncDiff`]
//! proposal. Differences the direction mode filters out never appear.
//!
//! v1 semantics (deliberate, documented):
//! - **Copy-only.** Nothing is ever deleted; mirror/delete semantics are a
//!   later phase.
//! - **Checksums later.** Identity is `(size, mtime)` — the checksum mode is
//!   reserved for a future `DiffReason`.
//! - **Conflicts need a human.** Same mtime with different size is
//!   [`DiffReason::Conflict`] — the executor skips it rather than guess.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use flow_core::{DiffReason, RemoteFile, SyncDiff, SyncDirection, TransferDirection};

/// How mtimes may drift while still counting as "same" — FTP timestamps are
/// second-granular, so sub-second noise must not create diffs.
pub const DEFAULT_MTIME_TOLERANCE_SECS: i64 = 2;

/// Compare tuning knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompareOptions {
    /// Which way changes may flow.
    pub direction: SyncDirection,
    /// Allowed mtime drift in seconds.
    pub mtime_tolerance_secs: i64,
}

impl CompareOptions {
    /// Options with the default tolerance.
    pub fn new(direction: SyncDirection) -> Self {
        Self {
            direction,
            mtime_tolerance_secs: DEFAULT_MTIME_TOLERANCE_SECS,
        }
    }
}

/// Compare two directory snapshots (relative path → entry) into a sync plan.
///
/// The maps are keyed by **relative path**; the caller walks both trees and
/// makes paths relative to the sync roots, so nesting compares correctly.
/// Directories are structural — they never produce diffs of their own.
///
/// Output is sorted by path for stable, preview-friendly output.
pub fn compare(
    local: &HashMap<String, RemoteFile>,
    remote: &HashMap<String, RemoteFile>,
    options: &CompareOptions,
) -> Vec<SyncDiff> {
    let mut diffs = Vec::new();
    let mut names: Vec<&String> = local.keys().chain(remote.keys()).collect();
    names.sort();
    names.dedup();

    for name in names {
        let in_local = local.get(name);
        let in_remote = remote.get(name);

        // Structural entries never produce proposals; their children were
        // walked instead.
        let is_directory = |entry: Option<&RemoteFile>| {
            entry.is_some_and(|entry| entry.kind == flow_core::FileKind::Directory)
        };
        if is_directory(in_local) || is_directory(in_remote) {
            continue;
        }

        match (in_local, in_remote) {
            // Only local → propose upload (unless remote-to-local).
            (Some(l), None) => {
                if flows_toward(options.direction, TransferDirection::Upload) {
                    diffs.push(SyncDiff {
                        path: name.clone(),
                        direction: TransferDirection::Upload,
                        size: l.size,
                        reason: DiffReason::Missing,
                    });
                }
            }

            // Only remote → propose download (unless local-to-remote).
            (None, Some(r)) => {
                if flows_toward(options.direction, TransferDirection::Download) {
                    diffs.push(SyncDiff {
                        path: name.clone(),
                        direction: TransferDirection::Download,
                        size: r.size,
                        reason: DiffReason::Missing,
                    });
                }
            }

            // Both sides — decide by (mtime, size).
            (Some(l), Some(r)) => {
                if let Some(diff) = decide_pair(name, l, r, options) {
                    diffs.push(diff);
                }
            }

            (None, None) => unreachable!("name came from one of the maps"),
        }
    }

    diffs
}

/// Decide what to do with an entry present on both sides.
fn decide_pair(
    name: &str,
    local: &RemoteFile,
    remote: &RemoteFile,
    options: &CompareOptions,
) -> Option<SyncDiff> {
    use flow_core::FileKind;

    // A directory on either side is structural; children were walked instead.
    if local.kind == FileKind::Directory || remote.kind == FileKind::Directory {
        return None;
    }

    let mtime_delta = local.modified - remote.modified;
    let same_mtime = mtime_delta.abs() <= options.mtime_tolerance_secs;
    let same_size = local.size == remote.size;

    if same_mtime && same_size {
        return None; // in sync
    }

    if same_mtime && !same_size {
        // Indeterminate: cannot tell which side to trust. Human decision.
        return Some(SyncDiff {
            path: name.to_string(),
            direction: TransferDirection::Upload,
            size: local.size,
            reason: DiffReason::Conflict,
        });
    }

    let (direction, size, reason) = if mtime_delta > 0 {
        // Local is newer.
        (TransferDirection::Upload, local.size, DiffReason::Newer)
    } else {
        (TransferDirection::Download, remote.size, DiffReason::Newer)
    };

    if !flows_toward(options.direction, direction) {
        return None;
    }

    Some(SyncDiff {
        path: name.to_string(),
        direction,
        size,
        reason,
    })
}

/// Whether a proposal's direction is allowed by the sync mode.
fn flows_toward(mode: SyncDirection, direction: TransferDirection) -> bool {
    match mode {
        SyncDirection::Bidirectional => true,
        SyncDirection::LocalToRemote => direction == TransferDirection::Upload,
        SyncDirection::RemoteToLocal => direction == TransferDirection::Download,
    }
}

/// Whether the executor may act on a diff without a human decision.
pub fn is_executable(diff: &SyncDiff) -> bool {
    diff.reason != DiffReason::Conflict
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::FileKind;

    fn file(name: &str, size: u64, modified: i64) -> RemoteFile {
        RemoteFile {
            name: name.rsplit('/').next().unwrap_or(name).to_string(),
            kind: FileKind::File,
            size,
            modified,
            permissions: None,
            owner: None,
            group: None,
        }
    }

    fn dir(name: &str) -> RemoteFile {
        RemoteFile {
            name: name.rsplit('/').next().unwrap_or(name).to_string(),
            kind: FileKind::Directory,
            size: 0,
            modified: 0,
            permissions: None,
            owner: None,
            group: None,
        }
    }

    fn snapshot(entries: Vec<RemoteFile>) -> HashMap<String, RemoteFile> {
        entries
            .into_iter()
            .map(|entry| (entry.name.clone(), entry))
            .collect()
    }

    // Note: compare keys by the map keys; the `name` field inside the entry is
    // cosmetic for these tests.

    #[test]
    fn empty_sides_produce_nothing() {
        let empty = HashMap::new();
        assert!(compare(&empty, &empty, &CompareOptions::new(SyncDirection::Bidirectional)).is_empty());
    }

    #[test]
    fn identical_snapshots_are_in_sync() {
        let local = snapshot(vec![file("a.txt", 100, 1_000)]);
        let remote = snapshot(vec![file("a.txt", 100, 1_000)]);
        assert!(compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional)).is_empty());
    }

    #[test]
    fn identical_within_tolerance_is_in_sync() {
        // 1 second apart, tolerance 2 → in sync.
        let local = snapshot(vec![file("a.txt", 100, 1_001)]);
        let remote = snapshot(vec![file("a.txt", 100, 1_000)]);
        assert!(compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional)).is_empty());
    }

    #[test]
    fn local_only_proposes_upload_missing() {
        let local = snapshot(vec![file("only-local.txt", 5, 1_000)]);
        let remote = HashMap::new();
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].direction, TransferDirection::Upload);
        assert_eq!(diffs[0].reason, DiffReason::Missing);
    }

    #[test]
    fn remote_only_proposes_download_missing() {
        let local = HashMap::new();
        let remote = snapshot(vec![file("only-remote.txt", 7, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].direction, TransferDirection::Download);
        assert_eq!(diffs[0].reason, DiffReason::Missing);
    }

    #[test]
    fn local_newer_uploads() {
        let local = snapshot(vec![file("a.txt", 120, 2_000)]);
        let remote = snapshot(vec![file("a.txt", 100, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].direction, TransferDirection::Upload);
        assert_eq!(diffs[0].reason, DiffReason::Newer);
        assert_eq!(diffs[0].size, 120);
    }

    #[test]
    fn remote_newer_downloads() {
        let local = snapshot(vec![file("a.txt", 100, 1_000)]);
        let remote = snapshot(vec![file("a.txt", 130, 3_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].direction, TransferDirection::Download);
        assert_eq!(diffs[0].reason, DiffReason::Newer);
    }

    #[test]
    fn same_mtime_different_size_is_conflict() {
        let local = snapshot(vec![file("a.txt", 100, 1_000)]);
        let remote = snapshot(vec![file("a.txt", 200, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].reason, DiffReason::Conflict);
    }

    #[test]
    fn conflicts_are_not_executable() {
        let local = snapshot(vec![file("a.txt", 100, 1_000)]);
        let remote = snapshot(vec![file("a.txt", 200, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert!(!is_executable(&diffs[0]));
        // Newer diffs are executable.
        let local = snapshot(vec![file("b.txt", 100, 2_000)]);
        let remote = snapshot(vec![file("b.txt", 100, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert!(is_executable(&diffs[0]));
    }

    #[test]
    fn local_to_remote_mode_filters_downloads() {
        let local = snapshot(vec![file("newer-local.txt", 10, 2_000), file("gone-local.txt", 10, 1_000)]);
        let remote = snapshot(vec![file("newer-local.txt", 10, 1_000), file("only-remote.txt", 10, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::LocalToRemote));
        // Only the upload proposals survive.
        assert!(diffs.iter().all(|d| d.direction == TransferDirection::Upload));
        assert_eq!(diffs.len(), 2); // newer-local + gone-local (missing remotely)
    }

    #[test]
    fn remote_to_local_mode_filters_uploads() {
        let local = snapshot(vec![file("newer-local.txt", 10, 2_000)]);
        let remote = snapshot(vec![file("newer-remote.txt", 10, 1_000), file("only-remote.txt", 10, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::RemoteToLocal));
        assert!(diffs.iter().all(|d| d.direction == TransferDirection::Download));
        assert_eq!(diffs.len(), 2); // newer-remote + only-remote
    }

    #[test]
    fn directories_never_propose() {
        let local = snapshot(vec![dir("sub"), file("a.txt", 1, 1_000)]);
        let remote = snapshot(vec![dir("sub"), file("a.txt", 2, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert_eq!(diffs.len(), 1); // just the size conflict on a.txt
        assert_eq!(diffs[0].path, "a.txt");
    }

    #[test]
    fn output_is_sorted_by_path() {
        let local = snapshot(vec![file("z.txt", 1, 2_000), file("a.txt", 1, 2_000)]);
        let remote = snapshot(vec![file("z.txt", 1, 1_000), file("a.txt", 1, 1_000)]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        let paths: Vec<&str> = diffs.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(paths, vec!["a.txt", "z.txt"]);
    }

    #[test]
    fn nested_relative_paths_compare_independently() {
        // Keyed by relative path (what the walker produces), not basename.
        let local = HashMap::from([
            ("assets/app.js".to_string(), file("app.js", 10, 2_000)),
            ("index.html".to_string(), file("index.html", 10, 1_000)),
        ]);
        let remote = HashMap::from([
            ("assets/app.js".to_string(), file("app.js", 10, 1_000)),
            ("index.html".to_string(), file("index.html", 10, 1_000)),
        ]);
        let diffs = compare(&local, &remote, &CompareOptions::new(SyncDirection::Bidirectional));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "assets/app.js");
        assert_eq!(diffs[0].direction, TransferDirection::Upload);
    }
}
