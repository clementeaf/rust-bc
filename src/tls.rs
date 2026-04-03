use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Carga certificados PEM desde un archivo.
fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsConfigError> {
    let file = fs::File::open(path)
        .map_err(|e| TlsConfigError::CertFile(path.display().to_string(), e))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TlsConfigError::CertParse(path.display().to_string(), e))
}

/// Carga la primera clave privada PEM (RSA, PKCS8 o EC) desde un archivo.
fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, TlsConfigError> {
    let file = fs::File::open(path)
        .map_err(|e| TlsConfigError::KeyFile(path.display().to_string(), e))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| TlsConfigError::KeyParse(path.display().to_string(), e))?
        .ok_or_else(|| TlsConfigError::NoKeyFound(path.display().to_string()))
}

/// Errores al construir la configuración TLS.
#[derive(Debug, thiserror::Error)]
pub enum TlsConfigError {
    #[error("cannot open cert file '{0}': {1}")]
    CertFile(String, std::io::Error),
    #[error("cannot parse certs in '{0}': {1}")]
    CertParse(String, std::io::Error),
    #[error("cannot open key file '{0}': {1}")]
    KeyFile(String, std::io::Error),
    #[error("cannot parse key in '{0}': {1}")]
    KeyParse(String, std::io::Error),
    #[error("no private key found in '{0}'")]
    NoKeyFound(String),
    #[error("rustls config error: {0}")]
    Rustls(#[from] rustls::Error),
}

/// Lee `TLS_CERT_PATH` y `TLS_KEY_PATH` del entorno.
/// - Si ambos están definidos, construye y devuelve `Ok(Some(ServerConfig))`.
/// - Si ninguno está definido, devuelve `Ok(None)` (sin TLS).
/// - Si solo uno está definido, devuelve error.
pub fn load_tls_config_from_env() -> Result<Option<ServerConfig>, TlsConfigError> {
    let cert_path = std::env::var("TLS_CERT_PATH").ok();
    let key_path = std::env::var("TLS_KEY_PATH").ok();

    match (cert_path, key_path) {
        (Some(cert), Some(key)) => {
            let config = build_server_config(Path::new(&cert), Path::new(&key))?;
            Ok(Some(config))
        }
        (None, None) => Ok(None),
        _ => Err(TlsConfigError::NoKeyFound(
            "both TLS_CERT_PATH and TLS_KEY_PATH must be set, or neither".into(),
        )),
    }
}

/// Construye un `rustls::ServerConfig` a partir de rutas a cert y key PEM.
pub fn build_server_config(
    cert_path: &Path,
    key_path: &Path,
) -> Result<ServerConfig, TlsConfigError> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(config)
}

/// Política de verificación del certificado del peer en conexiones TLS salientes.
#[derive(Debug, Clone)]
pub enum PeerVerification {
    /// Verificar el certificado del servidor.
    /// - `ca_cert_path: None` → usar las raíces WebPKI embebidas (Mozilla root store).
    /// - `ca_cert_path: Some(path)` → cargar un CA personalizado desde `path`.
    Full { ca_cert_path: Option<PathBuf> },
    /// No verificar el certificado del servidor.
    ///
    /// # Advertencia
    /// Solo para desarrollo/testing. Nunca usar en producción.
    Dangerous,
}

/// Verifier nulo — acepta cualquier certificado sin validar.
/// Solo para `PeerVerification::Dangerous`.
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

/// Construye un `rustls::ClientConfig` para conexiones TLS salientes.
pub fn build_client_config(verification: PeerVerification) -> Result<ClientConfig, TlsConfigError> {
    match verification {
        PeerVerification::Full { ca_cert_path: None } => {
            let root_store =
                RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            Ok(ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth())
        }
        PeerVerification::Full {
            ca_cert_path: Some(path),
        } => {
            let mut root_store = RootCertStore::empty();
            for cert in load_certs(&path)? {
                root_store.add(cert)?;
            }
            Ok(ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth())
        }
        PeerVerification::Dangerous => Ok(ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth()),
    }
}

/// Lee variables de entorno para construir un `ClientConfig` de salida.
///
/// - Devuelve `Ok(None)` si `TLS_CERT_PATH` o `TLS_KEY_PATH` no están definidos.
/// - `TLS_VERIFY_PEER=false` → deshabilita la verificación (solo dev/testing).
/// - `TLS_CA_CERT_PATH` → sobrescribe las raíces WebPKI con una CA personalizada.
pub fn load_client_config_from_env() -> Result<Option<ClientConfig>, TlsConfigError> {
    let tls_enabled =
        std::env::var("TLS_CERT_PATH").is_ok() && std::env::var("TLS_KEY_PATH").is_ok();
    if !tls_enabled {
        return Ok(None);
    }

    let verify_peer = std::env::var("TLS_VERIFY_PEER")
        .unwrap_or_else(|_| "true".into())
        .to_lowercase();
    let skip_verify = verify_peer.trim() == "false";

    let verification = if skip_verify {
        PeerVerification::Dangerous
    } else {
        let ca_cert_path = std::env::var("TLS_CA_CERT_PATH").ok().map(PathBuf::from);
        PeerVerification::Full { ca_cert_path }
    };

    build_client_config(verification).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Certificado y clave autofirmados generados para tests.
    // Generado con: openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1
    //               -keyout key.pem -out cert.pem -days 3650 -nodes -subj '/CN=test'
    const TEST_CERT_PEM: &str = include_str!("../tests/fixtures/test_cert.pem");
    const TEST_KEY_PEM: &str = include_str!("../tests/fixtures/test_key.pem");

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn build_server_config_with_valid_cert_and_key() {
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let result = build_server_config(cert.path(), key.path());
        assert!(result.is_ok());
    }

    #[test]
    fn build_server_config_fails_with_missing_cert() {
        let key = write_temp(TEST_KEY_PEM);
        let result = build_server_config(Path::new("/nonexistent/cert.pem"), key.path());
        assert!(matches!(result, Err(TlsConfigError::CertFile(..))));
    }

    #[test]
    fn build_server_config_fails_with_missing_key() {
        let cert = write_temp(TEST_CERT_PEM);
        let result = build_server_config(cert.path(), Path::new("/nonexistent/key.pem"));
        assert!(matches!(result, Err(TlsConfigError::KeyFile(..))));
    }

    #[test]
    fn build_server_config_fails_with_invalid_key_content() {
        let cert = write_temp(TEST_CERT_PEM);
        let bad_key = write_temp("not a key");
        let result = build_server_config(cert.path(), bad_key.path());
        assert!(matches!(result, Err(TlsConfigError::NoKeyFound(..))));
    }

    #[test]
    fn load_from_env_returns_none_when_no_vars() {
        // Limpiamos por si acaso
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        let result = load_tls_config_from_env().unwrap();
        assert!(result.is_none());
    }

    // ── ClientConfig tests ─────────────────────────────────────────────────

    #[test]
    fn build_client_config_full_with_webpki_roots() {
        let result = build_client_config(PeerVerification::Full { ca_cert_path: None });
        assert!(result.is_ok());
    }

    #[test]
    fn build_client_config_full_with_custom_ca() {
        let ca = write_temp(TEST_CERT_PEM);
        let result = build_client_config(PeerVerification::Full {
            ca_cert_path: Some(ca.path().to_path_buf()),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn build_client_config_full_with_missing_ca_file() {
        let result = build_client_config(PeerVerification::Full {
            ca_cert_path: Some(PathBuf::from("/nonexistent/ca.pem")),
        });
        assert!(matches!(result, Err(TlsConfigError::CertFile(..))));
    }

    #[test]
    fn build_client_config_dangerous_skips_verification() {
        let result = build_client_config(PeerVerification::Dangerous);
        assert!(result.is_ok());
    }

    #[test]
    fn load_client_config_from_env_returns_none_without_tls_vars() {
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_VERIFY_PEER");
        std::env::remove_var("TLS_CA_CERT_PATH");
        let result = load_client_config_from_env().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_client_config_from_env_full_when_tls_enabled() {
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::remove_var("TLS_VERIFY_PEER");
        std::env::remove_var("TLS_CA_CERT_PATH");
        let result = load_client_config_from_env();
        // Restore env
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn load_client_config_from_env_dangerous_when_verify_peer_false() {
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::set_var("TLS_VERIFY_PEER", "false");
        std::env::remove_var("TLS_CA_CERT_PATH");
        let result = load_client_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_VERIFY_PEER");
        assert!(result.unwrap().is_some());
    }
}
