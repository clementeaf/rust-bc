/// TLS P2P integration tests
///
/// Spins up a real TLS server and client on localhost using the test fixtures
/// (self-signed cert + key). The client uses `PeerVerification::Dangerous` so
/// the handshake succeeds without a trusted CA.
///
/// mTLS tests generate fresh certs via rcgen so that the SAN matches "localhost".

const TEST_CERT_PEM: &str = include_str!("fixtures/test_cert.pem");
const TEST_KEY_PEM: &str = include_str!("fixtures/test_key.pem");

use rust_bc::tls::{
    build_client_config, build_client_config_mtls, build_server_config,
    build_server_config_mtls, PeerVerification,
};
use rustls::pki_types::ServerName;
use rcgen::{generate_simple_self_signed, CertifiedKey};
use std::io::Write as IoWrite;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

fn write_temp(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

/// Spawn a TLS server that accepts one connection, reads bytes until EOF,
/// and returns them. Resolves once the connection closes.
async fn spawn_tls_server(
    listener: TcpListener,
    acceptor: TlsAcceptor,
) -> tokio::task::JoinHandle<Vec<u8>> {
    tokio::spawn(async move {
        let (tcp, _peer) = listener.accept().await.expect("server accept");
        let mut tls = acceptor.accept(tcp).await.expect("server TLS handshake");
        let mut buf = Vec::new();
        tls.read_to_end(&mut buf).await.expect("server read");
        buf
    })
}

#[tokio::test]
async fn tls_handshake_succeeds_between_client_and_server() {
    let cert_file = write_temp(TEST_CERT_PEM);
    let key_file = write_temp(TEST_KEY_PEM);

    // Build server TLS config
    let server_cfg = build_server_config(cert_file.path(), key_file.path())
        .expect("build_server_config");
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));

    // Bind on a random localhost port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let server_handle = spawn_tls_server(listener, acceptor).await;

    // Build client TLS config with Dangerous verifier (self-signed cert)
    let client_cfg = build_client_config(PeerVerification::Dangerous).expect("build_client_config");
    let connector = TlsConnector::from(Arc::new(client_cfg));

    let tcp = TcpStream::connect(local_addr).await.unwrap();
    let server_name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector
        .connect(server_name, tcp)
        .await
        .expect("client TLS handshake");

    // Send a message and close the write side
    tls.write_all(b"hello tls").await.unwrap();
    tls.shutdown().await.unwrap();

    // Server should have received the bytes
    let received = server_handle.await.expect("server task");
    assert_eq!(received, b"hello tls");
}

#[tokio::test]
async fn tls_server_rejects_plain_tcp_connection() {
    let cert_file = write_temp(TEST_CERT_PEM);
    let key_file = write_temp(TEST_KEY_PEM);

    let server_cfg = build_server_config(cert_file.path(), key_file.path())
        .expect("build_server_config");
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    // Server: accept one connection and try the TLS handshake — expect it to fail
    let server_handle = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.expect("accept");
        acceptor.accept(tcp).await.is_err()
    });

    // Connect with plain TCP and send raw bytes (no TLS handshake)
    let mut plain = TcpStream::connect(local_addr).await.unwrap();
    plain.write_all(b"not a tls handshake").await.unwrap();
    drop(plain);

    let handshake_failed = server_handle.await.expect("server task");
    assert!(handshake_failed, "server should reject plain TCP");
}

#[tokio::test]
async fn bidirectional_tls_exchange() {
    let cert_file = write_temp(TEST_CERT_PEM);
    let key_file = write_temp(TEST_KEY_PEM);

    let server_cfg = build_server_config(cert_file.path(), key_file.path())
        .expect("build_server_config");
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    // Server: echo back whatever it receives
    let server_handle = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.expect("accept");
        let mut tls = acceptor.accept(tcp).await.expect("TLS handshake");
        let mut buf = [0u8; 64];
        let n = tls.read(&mut buf).await.expect("read");
        tls.write_all(&buf[..n]).await.expect("write");
        tls.shutdown().await.expect("shutdown");
    });

    let client_cfg = build_client_config(PeerVerification::Dangerous).expect("build_client_config");
    let connector = TlsConnector::from(Arc::new(client_cfg));

    let tcp = TcpStream::connect(local_addr).await.unwrap();
    let server_name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector.connect(server_name, tcp).await.expect("client TLS");

    let msg = b"ping";
    tls.write_all(msg).await.unwrap();

    let mut response = vec![0u8; msg.len()];
    tls.read_exact(&mut response).await.unwrap();

    server_handle.await.expect("server task");

    assert_eq!(response, msg);
}

// ── mTLS integration tests ─────────────────────────────────────────────────

/// Generate a self-signed cert+key for "localhost" using rcgen, returning PEM strings.
fn gen_localhost_cert() -> (String, String) {
    let CertifiedKey { cert, key_pair } =
        generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    (cert.pem(), key_pair.serialize_pem())
}

/// Write PEM content to a temp file.
fn write_pem(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

/// mTLS handshake succeeds when both peers present certs signed by the shared CA.
#[tokio::test]
async fn mtls_handshake_succeeds_with_valid_client_cert() {
    let (cert_pem, key_pem) = gen_localhost_cert();

    let cert_file = write_pem(&cert_pem);
    let key_file = write_pem(&key_pem);
    // The self-signed cert IS the CA.
    let ca_file = write_pem(&cert_pem);

    let server_cfg =
        build_server_config_mtls(cert_file.path(), key_file.path(), ca_file.path())
            .expect("build_server_config_mtls");
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    let server_handle = spawn_tls_server(listener, acceptor).await;

    let client_cfg =
        build_client_config_mtls(cert_file.path(), key_file.path(), ca_file.path())
            .expect("build_client_config_mtls");
    let connector = TlsConnector::from(Arc::new(client_cfg));

    let tcp = TcpStream::connect(local_addr).await.unwrap();
    let server_name = ServerName::try_from("localhost").unwrap();
    let mut tls = connector
        .connect(server_name, tcp)
        .await
        .expect("mTLS client handshake");

    tls.write_all(b"mtls ok").await.unwrap();
    tls.shutdown().await.unwrap();

    let received = server_handle.await.expect("server task");
    assert_eq!(received, b"mtls ok");
}

/// mTLS server rejects a client that presents no certificate.
#[tokio::test]
async fn mtls_server_rejects_client_without_cert() {
    let (cert_pem, key_pem) = gen_localhost_cert();

    let cert_file = write_pem(&cert_pem);
    let key_file = write_pem(&key_pem);
    let ca_file = write_pem(&cert_pem);

    let server_cfg =
        build_server_config_mtls(cert_file.path(), key_file.path(), ca_file.path())
            .expect("build_server_config_mtls");
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    // Server: accept one connection, TLS handshake should fail (no client cert).
    let server_handle = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.expect("accept");
        acceptor.accept(tcp).await.is_err()
    });

    // Client: standard TLS without client cert.
    let client_cfg =
        build_client_config(PeerVerification::Dangerous).expect("build_client_config");
    let connector = TlsConnector::from(Arc::new(client_cfg));

    let tcp = TcpStream::connect(local_addr).await.unwrap();
    let server_name = ServerName::try_from("localhost").unwrap();
    // Client handshake may fail too — we only care that the server rejected it.
    let _ = connector.connect(server_name, tcp).await;

    let handshake_failed = server_handle.await.expect("server task");
    assert!(handshake_failed, "mTLS server must reject client without cert");
}
