use rustls::client::WebPkiServerVerifier;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
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
    #[error("mTLS requires TLS_CA_CERT_PATH to be set")]
    MtlsMissingCa,
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
    #[error("failed to build client cert verifier: {0}")]
    VerifierBuild(rustls::server::VerifierBuilderError),
    #[error("failed to build server cert verifier: {0}")]
    ServerVerifierBuild(#[from] rustls::client::VerifierBuilderError),
    #[error("invalid pin config: {0}")]
    PinConfig(PinConfigError),
}

/// Lee `TLS_CERT_PATH` y `TLS_KEY_PATH` del entorno.
/// - Si ambos están definidos, construye y devuelve `Ok(Some(ServerConfig))`.
/// - Si ninguno está definido, devuelve `Ok(None)` (sin TLS).
/// - Si solo uno está definido, devuelve error.
pub fn load_tls_config_from_env() -> Result<Option<ServerConfig>, TlsConfigError> {
    let cert_path = std::env::var("TLS_CERT_PATH").ok();
    let key_path = std::env::var("TLS_KEY_PATH").ok();

    let mutual = std::env::var("TLS_MUTUAL")
        .unwrap_or_default()
        .to_lowercase();
    let mtls = mutual.trim() == "true";

    match (cert_path, key_path) {
        (Some(cert), Some(key)) => {
            let config = if mtls {
                let ca = std::env::var("TLS_CA_CERT_PATH")
                    .map_err(|_| TlsConfigError::MtlsMissingCa)?;
                let pins = CertPinConfig::from_env().map_err(TlsConfigError::PinConfig)?;
                if pins.is_disabled() {
                    build_server_config_mtls(Path::new(&cert), Path::new(&key), Path::new(&ca))?
                } else {
                    // mTLS + pinning: build inline para envolver el verifier con PinningClientCertVerifier
                    let certs = load_certs(Path::new(&cert))?;
                    let key_der = load_private_key(Path::new(&key))?;
                    let mut root_store = RootCertStore::empty();
                    for c in load_certs(Path::new(&ca))? {
                        root_store.add(c)?;
                    }
                    let inner = WebPkiClientVerifier::builder(Arc::new(root_store))
                        .build()
                        .map_err(TlsConfigError::VerifierBuild)?;
                    let verifier = Arc::new(PinningClientCertVerifier::new(inner, pins));
                    ServerConfig::builder()
                        .with_client_cert_verifier(verifier)
                        .with_single_cert(certs, key_der)?
                }
            } else {
                build_server_config(Path::new(&cert), Path::new(&key))?
            };
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

/// Construye un `rustls::ServerConfig` con mTLS: exige que el cliente presente
/// un certificado firmado por la CA indicada en `ca_cert_path`.
pub fn build_server_config_mtls(
    cert_path: &Path,
    key_path: &Path,
    ca_cert_path: &Path,
) -> Result<ServerConfig, TlsConfigError> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let mut root_store = RootCertStore::empty();
    for cert in load_certs(ca_cert_path)? {
        root_store.add(cert)?;
    }

    let verifier = WebPkiClientVerifier::builder(Arc::new(root_store))
        .build()
        .map_err(TlsConfigError::VerifierBuild)?;

    let config = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
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

/// Construye un `rustls::ClientConfig` con mTLS: presenta cert+key propios al servidor
/// y verifica el cert del servidor contra `ca_cert_path`.
pub fn build_client_config_mtls(
    cert_path: &Path,
    key_path: &Path,
    ca_cert_path: &Path,
) -> Result<ClientConfig, TlsConfigError> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let mut root_store = RootCertStore::empty();
    for cert in load_certs(ca_cert_path)? {
        root_store.add(cert)?;
    }

    Ok(ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_client_auth_cert(certs, key)?)
}

/// Lee variables de entorno para construir un `ClientConfig` de salida.
///
/// - Devuelve `Ok(None)` si `TLS_CERT_PATH` o `TLS_KEY_PATH` no están definidos.
/// - `TLS_MUTUAL=true` → presenta cert cliente al servidor (mTLS); requiere `TLS_CA_CERT_PATH`.
/// - `TLS_VERIFY_PEER=false` → deshabilita la verificación (solo dev/testing).
/// - `TLS_CA_CERT_PATH` → sobrescribe las raíces WebPKI con una CA personalizada.
/// - `TLS_PINNED_CERTS` → fingerprints SHA-256 del cert del servidor (hex, separados por coma).
pub fn load_client_config_from_env() -> Result<Option<ClientConfig>, TlsConfigError> {
    let cert_path = std::env::var("TLS_CERT_PATH").ok();
    let key_path = std::env::var("TLS_KEY_PATH").ok();

    let tls_enabled = cert_path.is_some() && key_path.is_some();
    if !tls_enabled {
        return Ok(None);
    }

    let mutual = std::env::var("TLS_MUTUAL")
        .unwrap_or_default()
        .to_lowercase();
    let mtls = mutual.trim() == "true";

    let pins = CertPinConfig::from_env().map_err(TlsConfigError::PinConfig)?;

    if mtls {
        let cert = cert_path.unwrap();
        let key = key_path.unwrap();
        let ca = std::env::var("TLS_CA_CERT_PATH")
            .map_err(|_| TlsConfigError::MtlsMissingCa)?;

        if pins.is_disabled() {
            return build_client_config_mtls(Path::new(&cert), Path::new(&key), Path::new(&ca))
                .map(Some);
        }

        // mTLS + pinning: construir verifier de servidor con pin y adjuntar cert cliente
        let certs = load_certs(Path::new(&cert))?;
        let key_der = load_private_key(Path::new(&key))?;
        let mut root_store = RootCertStore::empty();
        for c in load_certs(Path::new(&ca))? {
            root_store.add(c)?;
        }
        let inner = WebPkiServerVerifier::builder(Arc::new(root_store)).build()?;
        let verifier = Arc::new(PinningServerCertVerifier::new(inner, pins));
        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(certs, key_der)?;
        return Ok(Some(config));
    }

    let verify_peer = std::env::var("TLS_VERIFY_PEER")
        .unwrap_or_else(|_| "true".into())
        .to_lowercase();
    let skip_verify = verify_peer.trim() == "false";

    // TLS_VERIFY_PEER=false → modo peligroso, el pinning no aplica
    if skip_verify {
        return build_client_config(PeerVerification::Dangerous).map(Some);
    }

    // TLS normal con o sin CA personalizada
    if pins.is_disabled() {
        let ca_cert_path = std::env::var("TLS_CA_CERT_PATH").ok().map(PathBuf::from);
        return build_client_config(PeerVerification::Full { ca_cert_path }).map(Some);
    }

    // TLS normal + pinning
    let ca_cert_path = std::env::var("TLS_CA_CERT_PATH").ok();
    let mut root_store = RootCertStore::empty();
    match ca_cert_path {
        Some(path) => {
            for c in load_certs(Path::new(&path))? {
                root_store.add(c)?;
            }
        }
        None => {
            root_store = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
    }
    let inner = WebPkiServerVerifier::builder(Arc::new(root_store)).build()?;
    let verifier = Arc::new(PinningServerCertVerifier::new(inner, pins));
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    Ok(Some(config))
}

// ── Certificate pinning ────────────────────────────────────────────────────

/// Error al cargar o parsear la configuración de pinning.
#[derive(Debug, thiserror::Error)]
pub enum PinConfigError {
    #[error("fingerprint '{0}' has invalid length (expected 64 hex chars)")]
    InvalidLength(String),
    #[error("fingerprint '{0}' is not valid hex: {1}")]
    InvalidHex(String, hex::FromHexError),
}

/// Conjunto de fingerprints SHA-256 de certificados permitidos.
///
/// Si el conjunto está vacío, el pinning está desactivado (acepta todos).
#[derive(Debug, Clone, Default)]
pub struct CertPinConfig {
    fingerprints: HashSet<[u8; 32]>,
}

impl CertPinConfig {
    /// Construye una config vacía (pinning desactivado).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Construye a partir de una lista de fingerprints SHA-256 en hex (64 caracteres c/u).
    pub fn from_fingerprints(
        hex_fingerprints: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, PinConfigError> {
        let mut fingerprints = HashSet::new();
        for raw in hex_fingerprints {
            let s = raw.as_ref().trim();
            if s.len() != 64 {
                return Err(PinConfigError::InvalidLength(s.to_string()));
            }
            let bytes = hex::decode(s)
                .map_err(|e| PinConfigError::InvalidHex(s.to_string(), e))?;
            let arr: [u8; 32] = bytes.try_into().expect("hex::decode returned 32 bytes");
            fingerprints.insert(arr);
        }
        Ok(Self { fingerprints })
    }

    /// Lee `TLS_PINNED_CERTS` del entorno (fingerprints separados por coma).
    ///
    /// - Variable ausente → `Ok(CertPinConfig::empty())` (pinning desactivado).
    /// - Variable presente pero vacía → `Ok(CertPinConfig::empty())`.
    pub fn from_env() -> Result<Self, PinConfigError> {
        let raw = match std::env::var("TLS_PINNED_CERTS") {
            Ok(v) => v,
            Err(_) => return Ok(Self::empty()),
        };
        let parts: Vec<&str> = raw.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Ok(Self::empty());
        }
        Self::from_fingerprints(parts)
    }

    /// `true` si no hay pins configurados (acepta cualquier certificado).
    pub fn is_disabled(&self) -> bool {
        self.fingerprints.is_empty()
    }

    /// `true` si el certificado está en la allowlist (o si el pinning está desactivado).
    pub fn is_allowed(&self, cert: &CertificateDer<'_>) -> bool {
        if self.is_disabled() {
            return true;
        }
        self.fingerprints.contains(&cert_fingerprint(cert))
    }
}

/// Calcula el fingerprint SHA-256 de los bytes DER del certificado.
pub fn cert_fingerprint(cert: &CertificateDer<'_>) -> [u8; 32] {
    Sha256::digest(cert.as_ref()).into()
}

// ── Verifiers con pinning ──────────────────────────────────────────────────

/// Verifica el cert del **servidor** (uso en `ClientConfig`) aplicando primero
/// la validación CA del verifier subyacente y luego comprobando el fingerprint.
#[derive(Debug)]
pub struct PinningServerCertVerifier {
    inner: Arc<dyn rustls::client::danger::ServerCertVerifier>,
    pins: CertPinConfig,
}

impl PinningServerCertVerifier {
    pub fn new(
        inner: Arc<dyn rustls::client::danger::ServerCertVerifier>,
        pins: CertPinConfig,
    ) -> Self {
        Self { inner, pins }
    }
}

impl rustls::client::danger::ServerCertVerifier for PinningServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &rustls::pki_types::ServerName<'_>,
        ocsp_response: &[u8],
        now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        self.inner
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)?;
        if !self.pins.is_allowed(end_entity) {
            return Err(rustls::Error::General(format!(
                "server cert fingerprint not in allowlist: {}",
                hex::encode(cert_fingerprint(end_entity))
            )));
        }
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Verifica el cert del **cliente** en mTLS (uso en `ServerConfig`) aplicando
/// primero la validación CA del verifier subyacente y luego el fingerprint.
#[derive(Debug)]
pub struct PinningClientCertVerifier {
    inner: Arc<dyn rustls::server::danger::ClientCertVerifier>,
    pins: CertPinConfig,
}

impl PinningClientCertVerifier {
    pub fn new(
        inner: Arc<dyn rustls::server::danger::ClientCertVerifier>,
        pins: CertPinConfig,
    ) -> Self {
        Self { inner, pins }
    }
}

impl rustls::server::danger::ClientCertVerifier for PinningClientCertVerifier {
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        self.inner.root_hint_subjects()
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::server::danger::ClientCertVerified, rustls::Error> {
        self.inner.verify_client_cert(end_entity, intermediates, now)?;
        if !self.pins.is_allowed(end_entity) {
            return Err(rustls::Error::General(format!(
                "client cert fingerprint not in allowlist: {}",
                hex::encode(cert_fingerprint(end_entity))
            )));
        }
        Ok(rustls::server::danger::ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::NamedTempFile;

    /// Serializa todos los tests que leen/escriben env vars de TLS.
    /// Las env vars son estado global del proceso; sin esta guardia los tests
    /// en paralelo se interfieren entre sí.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
        let _g = ENV_LOCK.lock().unwrap();
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
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_VERIFY_PEER");
        std::env::remove_var("TLS_CA_CERT_PATH");
        std::env::remove_var("TLS_PINNED_CERTS");
        let result = load_client_config_from_env().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_client_config_from_env_full_when_tls_enabled() {
        let _g = ENV_LOCK.lock().unwrap();
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::remove_var("TLS_MUTUAL");
        std::env::remove_var("TLS_VERIFY_PEER");
        std::env::remove_var("TLS_CA_CERT_PATH");
        std::env::remove_var("TLS_PINNED_CERTS");
        let result = load_client_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn load_client_config_from_env_dangerous_when_verify_peer_false() {
        let _g = ENV_LOCK.lock().unwrap();
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::remove_var("TLS_MUTUAL");
        std::env::set_var("TLS_VERIFY_PEER", "false");
        std::env::remove_var("TLS_CA_CERT_PATH");
        std::env::remove_var("TLS_PINNED_CERTS");
        let result = load_client_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_VERIFY_PEER");
        assert!(result.unwrap().is_some());
    }

    // ── mTLS unit tests ────────────────────────────────────────────────────

    #[test]
    fn build_server_config_mtls_accepts_valid_ca() {
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let ca = write_temp(TEST_CERT_PEM);
        let result = build_server_config_mtls(cert.path(), key.path(), ca.path());
        assert!(result.is_ok());
    }

    #[test]
    fn build_server_config_mtls_fails_with_missing_ca() {
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let result = build_server_config_mtls(
            cert.path(),
            key.path(),
            Path::new("/nonexistent/ca.pem"),
        );
        assert!(matches!(result, Err(TlsConfigError::CertFile(..))));
    }

    #[test]
    fn build_client_config_mtls_accepts_valid_cert_and_ca() {
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let ca = write_temp(TEST_CERT_PEM);
        let result = build_client_config_mtls(cert.path(), key.path(), ca.path());
        assert!(result.is_ok());
    }

    #[test]
    fn build_client_config_mtls_fails_with_missing_ca() {
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let result = build_client_config_mtls(
            cert.path(),
            key.path(),
            Path::new("/nonexistent/ca.pem"),
        );
        assert!(matches!(result, Err(TlsConfigError::CertFile(..))));
    }

    #[test]
    fn load_tls_config_from_env_mtls_fails_without_ca() {
        let _g = ENV_LOCK.lock().unwrap();
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::set_var("TLS_MUTUAL", "true");
        std::env::remove_var("TLS_CA_CERT_PATH");
        std::env::remove_var("TLS_PINNED_CERTS");
        let result = load_tls_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_MUTUAL");
        assert!(matches!(result, Err(TlsConfigError::MtlsMissingCa)));
    }

    #[test]
    fn load_tls_config_from_env_mtls_succeeds_with_ca() {
        let _g = ENV_LOCK.lock().unwrap();
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let ca = write_temp(TEST_CERT_PEM);
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::set_var("TLS_MUTUAL", "true");
        std::env::set_var("TLS_CA_CERT_PATH", ca.path());
        std::env::remove_var("TLS_PINNED_CERTS");
        let result = load_tls_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_MUTUAL");
        std::env::remove_var("TLS_CA_CERT_PATH");
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn load_client_config_from_env_mtls_fails_without_ca() {
        let _g = ENV_LOCK.lock().unwrap();
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::set_var("TLS_MUTUAL", "true");
        std::env::remove_var("TLS_CA_CERT_PATH");
        std::env::remove_var("TLS_PINNED_CERTS");
        let result = load_client_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_MUTUAL");
        assert!(matches!(result, Err(TlsConfigError::MtlsMissingCa)));
    }

    #[test]
    fn load_client_config_from_env_mtls_succeeds_with_ca() {
        let _g = ENV_LOCK.lock().unwrap();
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let ca = write_temp(TEST_CERT_PEM);
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::set_var("TLS_MUTUAL", "true");
        std::env::set_var("TLS_CA_CERT_PATH", ca.path());
        std::env::remove_var("TLS_PINNED_CERTS");
        let result = load_client_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_MUTUAL");
        std::env::remove_var("TLS_CA_CERT_PATH");
        assert!(result.unwrap().is_some());
    }

    // ── Certificate pinning unit tests ────────────────────────────────────

    /// Calcula el fingerprint del TEST_CERT_PEM real (cargado como DER).
    fn test_cert_fingerprint() -> [u8; 32] {
        let cert_file = write_temp(TEST_CERT_PEM);
        let certs = load_certs(cert_file.path()).unwrap();
        cert_fingerprint(&certs[0])
    }

    #[test]
    fn cert_pin_config_empty_allows_any_cert() {
        let config = CertPinConfig::empty();
        assert!(config.is_disabled());
        let cert_file = write_temp(TEST_CERT_PEM);
        let certs = load_certs(cert_file.path()).unwrap();
        assert!(config.is_allowed(&certs[0]));
    }

    #[test]
    fn cert_pin_config_allows_pinned_cert() {
        let fp = test_cert_fingerprint();
        let hex = hex::encode(fp);
        let config = CertPinConfig::from_fingerprints([hex]).unwrap();
        assert!(!config.is_disabled());

        let cert_file = write_temp(TEST_CERT_PEM);
        let certs = load_certs(cert_file.path()).unwrap();
        assert!(config.is_allowed(&certs[0]));
    }

    #[test]
    fn cert_pin_config_rejects_unpinned_cert() {
        // Pin un fingerprint aleatorio que no coincide con el cert de test
        let wrong_hex = "a".repeat(64);
        let config = CertPinConfig::from_fingerprints([wrong_hex]).unwrap();

        let cert_file = write_temp(TEST_CERT_PEM);
        let certs = load_certs(cert_file.path()).unwrap();
        assert!(!config.is_allowed(&certs[0]));
    }

    #[test]
    fn cert_pin_config_rejects_invalid_hex() {
        let result = CertPinConfig::from_fingerprints(["zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"]);
        assert!(matches!(result, Err(PinConfigError::InvalidHex(..))));
    }

    #[test]
    fn cert_pin_config_rejects_wrong_length() {
        let result = CertPinConfig::from_fingerprints(["abcd"]);
        assert!(matches!(result, Err(PinConfigError::InvalidLength(..))));
    }

    #[test]
    fn cert_pin_config_from_env_disabled_when_var_absent() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("TLS_PINNED_CERTS");
        let config = CertPinConfig::from_env().unwrap();
        assert!(config.is_disabled());
    }

    #[test]
    fn cert_pin_config_from_env_disabled_when_var_empty() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("TLS_PINNED_CERTS", "");
        let config = CertPinConfig::from_env().unwrap();
        std::env::remove_var("TLS_PINNED_CERTS");
        assert!(config.is_disabled());
    }

    #[test]
    fn cert_pin_config_from_env_loads_valid_fingerprint() {
        let _g = ENV_LOCK.lock().unwrap();
        let fp = test_cert_fingerprint();
        let hex = hex::encode(fp);
        std::env::set_var("TLS_PINNED_CERTS", &hex);
        let config = CertPinConfig::from_env().unwrap();
        std::env::remove_var("TLS_PINNED_CERTS");

        assert!(!config.is_disabled());
        let cert_file = write_temp(TEST_CERT_PEM);
        let certs = load_certs(cert_file.path()).unwrap();
        assert!(config.is_allowed(&certs[0]));
    }

    #[test]
    fn cert_pin_config_from_env_errors_on_invalid_hex() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("TLS_PINNED_CERTS", "zz".repeat(32));
        let result = CertPinConfig::from_env();
        std::env::remove_var("TLS_PINNED_CERTS");
        assert!(matches!(result, Err(PinConfigError::InvalidHex(..))));
    }

    #[test]
    fn load_tls_config_from_env_mtls_with_pin_accepts_pinned_cert() {
        let _g = ENV_LOCK.lock().unwrap();
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let ca = write_temp(TEST_CERT_PEM);
        let fp = hex::encode(test_cert_fingerprint());
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::set_var("TLS_MUTUAL", "true");
        std::env::set_var("TLS_CA_CERT_PATH", ca.path());
        std::env::set_var("TLS_PINNED_CERTS", &fp);
        let result = load_tls_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_MUTUAL");
        std::env::remove_var("TLS_CA_CERT_PATH");
        std::env::remove_var("TLS_PINNED_CERTS");
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn load_client_config_from_env_with_pin_builds_config() {
        let _g = ENV_LOCK.lock().unwrap();
        let cert = write_temp(TEST_CERT_PEM);
        let key = write_temp(TEST_KEY_PEM);
        let ca = write_temp(TEST_CERT_PEM);
        let fp = hex::encode(test_cert_fingerprint());
        std::env::set_var("TLS_CERT_PATH", cert.path());
        std::env::set_var("TLS_KEY_PATH", key.path());
        std::env::remove_var("TLS_MUTUAL");
        std::env::set_var("TLS_CA_CERT_PATH", ca.path());
        std::env::set_var("TLS_PINNED_CERTS", &fp);
        let result = load_client_config_from_env();
        std::env::remove_var("TLS_CERT_PATH");
        std::env::remove_var("TLS_KEY_PATH");
        std::env::remove_var("TLS_CA_CERT_PATH");
        std::env::remove_var("TLS_PINNED_CERTS");
        assert!(result.unwrap().is_some());
    }
}
