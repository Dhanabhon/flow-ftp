//! Live-server integration test for the SFTP adapter.
//!
//! Ignored by default: it dials a real SSH server. Provide credentials and
//! run manually:
//!
//! ```sh
//! FLOW_SFTP_HOST=sftp.example.com \
//! FLOW_SFTP_PORT=22 \
//! FLOW_SFTP_USER=deploy \
//! FLOW_SFTP_PASS=secret \
//! cargo test -p flow-protocols --test live_sftp -- --ignored
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

use flow_core::{Credentials, CoreError, FilePath, Protocol, RemoteFs, TransferId};
use flow_protocols::{HostKeyPolicy, SftpFs};

/// Credentials from the environment, or `None` (test then skips).
fn credentials() -> Option<Credentials> {
    let host = std::env::var("FLOW_SFTP_HOST").ok()?;
    let user = std::env::var("FLOW_SFTP_USER").unwrap_or_else(|_| "root".into());
    let pass = std::env::var("FLOW_SFTP_PASS").unwrap_or_default();

    let mut creds = Credentials::new(host, user, Protocol::Sftp);
    if let Ok(port) = std::env::var("FLOW_SFTP_PORT") {
        creds = creds.with_port(port.parse().expect("FLOW_SFTP_PORT must be a number"));
    }
    if !pass.is_empty() {
        creds = creds.with_password(pass);
    }
    Some(creds)
}

/// Host key policy from the environment: `FLOW_SFTP_FINGERPRINT=SHA256:…`
/// pins the key; unset accepts any (development mode).
fn policy() -> HostKeyPolicy {
    match std::env::var("FLOW_SFTP_FINGERPRINT") {
        Ok(fp) if !fp.is_empty() => HostKeyPolicy::Pin { sha256: fp },
        _ => HostKeyPolicy::AcceptAny,
    }
}

/// Unique scratch directory name for this run.
fn scratch_name() -> String {
    format!(
        "flow-sftp-it-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_millis()
    )
}

#[tokio::test]
#[ignore = "dials a live SFTP server; configure via FLOW_SFTP_* env vars"]
async fn live_server_round_trip() {
    let Some(creds) = credentials() else {
        eprintln!("FLOW_SFTP_HOST not set; skipping");
        return;
    };
    eprintln!("connecting to {}:{} (sftp)", creds.host, creds.port);

    let mut fs = SftpFs::with_policy(policy());
    fs.connect(&creds).await.expect("SFTP connect failed");

    let dir = FilePath::new(scratch_name());

    fs.mkdir(&dir).await.expect("mkdir failed");
    let listing = fs.list(&dir).await.expect("list failed");
    assert!(listing.is_empty(), "fresh directory should be empty");

    let payload = b"flowftp sftp integration test";
    let local = std::env::temp_dir().join("flow-sftp-it-upload.txt");
    std::fs::write(&local, payload).expect("write local failed");
    let remote = dir.join("upload.txt");
    fs.upload(
        TransferId::new("it1"),
        &FilePath::new(local.to_string_lossy().into_owned()),
        &remote,
    )
    .await
    .expect("upload failed");

    let stat = fs.stat(&remote).await.expect("stat failed");
    assert_eq!(stat.size, payload.len() as u64, "uploaded size mismatch");
    assert_eq!(stat.kind, flow_core::FileKind::File);

    let listing = fs.list(&dir).await.expect("list after upload failed");
    assert_eq!(listing.len(), 1);
    assert_eq!(listing[0].name, "upload.txt");

    let download_path = std::env::temp_dir().join("flow-sftp-it-download.txt");
    fs.download(
        TransferId::new("it2"),
        &remote,
        &FilePath::new(download_path.to_string_lossy().into_owned()),
    )
    .await
    .expect("download failed");
    let round = std::fs::read(&download_path).expect("read downloaded failed");
    assert_eq!(round, payload);

    fs.rename(&remote, &dir.join("renamed.txt"))
        .await
        .expect("rename failed");
    fs.delete(&dir.join("renamed.txt"))
        .await
        .expect("delete failed");
    fs.delete(&dir).await.expect("rmdir failed");

    fs.disconnect().await.expect("disconnect failed");
}

#[tokio::test]
#[ignore = "dials a live SFTP server; configure via FLOW_SFTP_* env vars"]
async fn live_server_rejects_bad_credentials() {
    let Some(creds) = credentials() else {
        eprintln!("FLOW_SFTP_HOST not set; skipping");
        return;
    };
    let bad = creds.clone().with_password("definitely-not-the-password");

    let mut fs = SftpFs::with_policy(policy());
    let err = match fs.connect(&bad).await {
        Err(e) => e,
        Ok(()) => panic!("connect with wrong password must fail"),
    };
    assert!(
        matches!(err, CoreError::AuthFailed { .. }),
        "expected AuthFailed, got {err:?}"
    );
}
