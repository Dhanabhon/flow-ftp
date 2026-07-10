//! Integration tests for the `flow-core` public API.
//!
//! These exercise the crate the way an external consumer (the Application
//! Layer, a future protocol adapter) would: through its re-exported public
//! types only.

use flow_core::{
    Credentials, FilePath, FileKind, Protocol, RemoteFile, SyncDiff, SyncDirection,
    TransferDirection, TransferId, TransferProgress,
};

#[test]
fn credentials_redact_password_on_debug() {
    let c = Credentials::new("example.com", "tom", Protocol::Sftp).with_password("hunter2");
    let debug = format!("{c:?}");
    assert!(!debug.contains("hunter2"));
}

#[test]
fn filepath_parent_chain_reaches_root() {
    let mut p = FilePath::new("/a/b/c");
    p = p.parent();
    assert_eq!(p.as_str(), "/a/b");
    p = p.parent();
    assert_eq!(p.as_str(), "/a");
    p = p.parent();
    assert_eq!(p.as_str(), "/");
    // Root stays root.
    p = p.parent();
    assert_eq!(p.as_str(), "/");
}

#[test]
fn filepath_join_and_name() {
    let p = FilePath::new("/var/log").join("nginx");
    assert_eq!(p.as_str(), "/var/log/nginx");
    assert_eq!(p.name(), "nginx");
}

#[test]
fn remote_file_parent_entry_is_directory() {
    let e = RemoteFile::parent_entry();
    assert_eq!(e.kind, FileKind::Directory);
    assert_eq!(e.name, "..");
}

#[test]
fn transfer_progress_percent_and_remaining() {
    let p = TransferProgress { transferred: 250, total: 1000 };
    assert_eq!(p.percent(), 25);
    assert_eq!(p.remaining(), 750);
}

#[test]
fn transfer_progress_unknown_total() {
    let p = TransferProgress { transferred: 100, total: 0 };
    assert_eq!(p.percent(), 0);
    assert_eq!(p.fraction(), 0.0);
}

#[test]
fn sync_diff_round_trip_matches_frontend_shape() {
    let d = SyncDiff {
        path: "/public/index.html".into(),
        direction: TransferDirection::Upload,
        size: 14_200,
        reason: flow_core::DiffReason::Newer,
    };
    let json = serde_json::to_string(&d).unwrap();
    // Frontend reads direction as lowercase and reason as lowercase.
    assert!(json.contains("\"upload\""));
    assert!(json.contains("\"newer\""));
}

#[test]
fn sync_direction_frontend_strings() {
    assert_eq!(
        serde_json::to_string(&SyncDirection::Bidirectional).unwrap(),
        "\"both\""
    );
    assert_eq!(
        serde_json::to_string(&SyncDirection::RemoteToLocal).unwrap(),
        "\"remote-to-local\""
    );
}

#[test]
fn transfer_id_newtype_transparent() {
    let id = TransferId::new("t1");
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"t1\"");
}
