//! Live-server integration tests for the FTP/FTPS adapters.
//!
//! Ignored by default: they dial a real server. Provide credentials via
//! environment variables and run manually:
//!
//! ```sh
//! FLOW_FTP_HOST=ftp.example.com \
//! FLOW_FTP_PORT=21 \
//! FLOW_FTP_USER=deploy \
//! FLOW_FTP_PASS=secret \
//! cargo test -p flow-protocols --test live_ftp -- --ignored
//! ```
//!
//! For FTPS set `FLOW_FTP_TLS=explicit` (or `implicit`) and the matching port.

use std::time::{SystemTime, UNIX_EPOCH};

use flow_core::{Credentials, FilePath, Protocol, RemoteFs, TransferId};
use flow_protocols::{FtpsFs, FtpsMode, FtpFs};

/// Credentials from the environment, or `None` (test then skips).
fn credentials() -> Option<Credentials> {
    let host = std::env::var("FLOW_FTP_HOST").ok()?;
    let user = std::env::var("FLOW_FTP_USER").unwrap_or_else(|_| "anonymous".into());
    let pass = std::env::var("FLOW_FTP_PASS").unwrap_or_default();
    let tls = std::env::var("FLOW_FTP_TLS").unwrap_or_default();
    let protocol = if tls.is_empty() {
        Protocol::Ftp
    } else {
        Protocol::Ftps
    };

    let mut creds = Credentials::new(host, user, protocol);
    if let Ok(port) = std::env::var("FLOW_FTP_PORT") {
        creds = creds.with_port(port.parse().expect("FLOW_FTP_PORT must be a number"));
    }
    if !pass.is_empty() {
        creds = creds.with_password(pass);
    }
    Some(creds)
}

/// Which adapter to build, resolved from the environment.
enum Adapter {
    Plain,
    Tls(FtpsMode),
}

fn adapter() -> Adapter {
    match std::env::var("FLOW_FTP_TLS").unwrap_or_default().as_str() {
        "" => Adapter::Plain,
        "implicit" => Adapter::Tls(FtpsMode::Implicit),
        "explicit" => Adapter::Tls(FtpsMode::Explicit),
        other => panic!("FLOW_FTP_TLS must be 'explicit' or 'implicit', got '{other}'"),
    }
}

/// Unique scratch directory name for this run.
fn scratch_name() -> String {
    format!(
        "flow-ftp-it-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_millis()
    )
}

/// Connect, then run the full mkdir/upload/stat/download/rename/delete
/// round trip against whichever adapter was requested.
///
/// The two adapters are distinct concrete types (plain vs TLS stream), so the
/// scenario body is generated once per arm.
macro_rules! with_adapter {
    ($creds:expr, | $fs:ident | $body:block) => {
        match adapter() {
            Adapter::Plain => {
                let mut $fs = FtpFs::new();
                $fs.connect(&$creds).await.expect("FTP connect failed");
                let $fs = &mut $fs;
                $body
            }
            Adapter::Tls(mode) => {
                let mut $fs = match mode {
                    FtpsMode::Implicit => FtpsFs::implicit(),
                    FtpsMode::Explicit => FtpsFs::explicit(),
                };
                $fs.connect(&$creds).await.expect("FTPS connect failed");
                let $fs = &mut $fs;
                $body
            }
        }
    };
}

#[tokio::test]
#[ignore = "dials a live FTP(S) server; configure via FLOW_FTP_* env vars"]
async fn live_server_round_trip() {
    let Some(creds) = credentials() else {
        eprintln!("FLOW_FTP_HOST not set; skipping");
        return;
    };
    eprintln!(
        "connecting to {}:{} ({:?})",
        creds.host, creds.port, creds.protocol
    );

    with_adapter!(creds, |fs| {
        let dir = FilePath::new(scratch_name());

        fs.mkdir(&dir).await.expect("mkdir failed");
        fs.list(&dir).await.expect("list failed");

        let payload = b"flowftp integration test";
        let local = std::env::temp_dir().join("flow-ftp-it-upload.txt");
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

        let download_path = std::env::temp_dir().join("flow-ftp-it-download.txt");
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
    });
}

#[tokio::test]
#[ignore = "dials a live FTP(S) server; configure via FLOW_FTP_* env vars"]
async fn live_server_rejects_bad_credentials() {
    let Some(creds) = credentials() else {
        eprintln!("FLOW_FTP_HOST not set; skipping");
        return;
    };
    let bad = creds.clone().with_password("definitely-not-the-password");

    with_adapter!(bad, |fs| {
        let err = match fs.connect(&bad).await {
            Err(e) => e,
            Ok(()) => panic!("connect with wrong password must fail"),
        };
        assert!(
            matches!(err, flow_core::CoreError::AuthFailed { .. }),
            "expected AuthFailed, got {err:?}"
        );
    });
}
