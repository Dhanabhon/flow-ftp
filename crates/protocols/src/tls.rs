//! TLS client configuration for the FTPS adapters.
//!
//! Server certificates are validated against the **platform trust store**
//! (`rustls-platform-verifier` → Security.framework on macOS), so corporate
//! proxies and user-installed roots behave exactly as they do in Safari.

use std::sync::Arc;

use flow_core::{CoreError, CoreResult};
use rustls_platform_verifier::ConfigVerifierExt;
use suppaftp::tokio::AsyncRustlsConnector;

/// Build a TLS connector that validates servers against the OS trust store.
pub(crate) fn tls_connector() -> CoreResult<AsyncRustlsConnector> {
    let config = rustls::ClientConfig::with_platform_verifier()
        .map_err(|e| CoreError::Protocol(format!("failed to initialize TLS verifier: {e}")))?;
    Ok(AsyncRustlsConnector::from(tokio_rustls::TlsConnector::from(
        Arc::new(config),
    )))
}
