use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use std::fs;
use std::io::BufReader;
use std::path::Path;

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
}
