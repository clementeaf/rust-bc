use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::LocalBoxFuture;
use std::rc::Rc;
use uuid::Uuid;

/// Middleware that injects correlation ID into each request
pub struct CorrelationIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for CorrelationIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = CorrelationIdMiddlewareService<S>;
    type Future = futures::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        futures::future::ok(CorrelationIdMiddlewareService {
            service: Rc::new(service),
        })
    }
}

pub struct CorrelationIdMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for CorrelationIdMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let trace_id = req
            .headers()
            .get("X-Trace-ID")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        req.extensions_mut().insert(trace_id);

        let srv = self.service.clone();

        Box::pin(async move {
            let res = srv.call(req).await?;
            Ok(res)
        })
    }
}

// ── TLS Identity Middleware ───────────────────────────────────────────────────

// Re-export the canonical TlsIdentity from errors.rs — enforce_acl reads this type.
use crate::api::errors::TlsIdentity;

/// DER-encoded peer certificates extracted during the TLS handshake.
///
/// Stored in connection-level extensions via `HttpServer::on_connect`.
/// The middleware reads these to parse X.509 identity.
#[derive(Debug, Clone)]
pub struct PeerCertificates(pub Vec<Vec<u8>>);

/// Middleware that extracts the client certificate CN/O from a mTLS connection
/// and inserts a [`TlsIdentity`] into the request extensions.
///
/// When TLS is not configured or no client cert is presented, the request
/// proceeds without a `TlsIdentity` (handlers can check for its presence).
pub struct TlsIdentityMiddleware;

impl<S, B> Transform<S, ServiceRequest> for TlsIdentityMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TlsIdentityMiddlewareService<S>;
    type Future = futures::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        futures::future::ok(TlsIdentityMiddlewareService {
            service: Rc::new(service),
        })
    }
}

pub struct TlsIdentityMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for TlsIdentityMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Actix-web exposes the peer certificate chain via the connection info.
        // Extract CN and O from the first certificate's subject.
        if let Some(identity) = extract_tls_identity(&req) {
            req.extensions_mut().insert(identity);
        }

        let srv = self.service.clone();
        Box::pin(async move { srv.call(req).await })
    }
}

fn extract_tls_identity(req: &ServiceRequest) -> Option<TlsIdentity> {
    // 1. Try real X.509 peer certificates from the mTLS handshake.
    if let Some(peer_certs) = req.conn_data::<PeerCertificates>() {
        if let Some(der) = peer_certs.0.first() {
            if let Some(identity) = parse_x509_identity(der) {
                return Some(identity);
            }
        }
    }

    // 2. Fallback: X-TLS-Client-CN/O headers (set by TLS-terminating proxies or tests).
    let cn = req
        .headers()
        .get("X-TLS-Client-CN")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let org = req
        .headers()
        .get("X-TLS-Client-O")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let org_id = org.or_else(|| cn.clone())?;
    Some(TlsIdentity { org_id, role: None })
}

/// Parse CN and O from a DER-encoded X.509 certificate.
///
/// Maps X.509 subject fields to the ACL identity model:
///   - Organization (O) → `org_id` (used for endorsement policy checks)
///   - Common Name (CN) → used to infer MSP role:
///     - CN containing "admin" → `MspRole::Admin`
///     - CN containing "peer" or "orderer" → `MspRole::Peer`
///     - Otherwise → `MspRole::Client`
fn parse_x509_identity(der: &[u8]) -> Option<TlsIdentity> {
    use x509_parser::prelude::*;

    let (_, cert) = X509Certificate::from_der(der).ok()?;
    let subject = cert.subject();

    let org = subject
        .iter_organization()
        .next()
        .and_then(|attr| attr.as_str().ok())
        .map(|s| s.to_string());

    // org_id is the Organization field; fall back to CN if O is absent.
    let cn = subject
        .iter_common_name()
        .next()
        .and_then(|attr| attr.as_str().ok())
        .map(|s| s.to_string());

    let org_id = org.or_else(|| cn.clone())?;

    // Infer MSP role from the CN.
    let role = cn.as_deref().map(|cn_str| {
        let lower = cn_str.to_lowercase();
        if lower.contains("admin") {
            crate::msp::MspRole::Admin
        } else if lower.contains("peer") || lower.contains("orderer") {
            crate::msp::MspRole::Peer
        } else {
            crate::msp::MspRole::Client
        }
    });

    Some(TlsIdentity { org_id, role })
}

// ── Audit Middleware ─────────────────────────────────────────────────────────

/// Middleware that records every HTTP request to the audit store.
///
/// Captures: timestamp, method, path, org_id (from TlsIdentity or X-Org-Id),
/// source IP, response status code, trace_id, and duration in milliseconds.
pub struct AuditMiddleware {
    pub store: std::sync::Arc<dyn crate::audit::AuditStore>,
}

impl<S, B> Transform<S, ServiceRequest> for AuditMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuditMiddlewareService<S>;
    type Future = futures::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        futures::future::ok(AuditMiddlewareService {
            service: Rc::new(service),
            store: self.store.clone(),
        })
    }
}

pub struct AuditMiddlewareService<S> {
    service: Rc<S>,
    store: std::sync::Arc<dyn crate::audit::AuditStore>,
}

impl<S, B> Service<ServiceRequest> for AuditMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();
        let source_ip = req
            .peer_addr()
            .map(|a| a.ip().to_string())
            .unwrap_or_default();
        let trace_id = req
            .headers()
            .get("X-Trace-ID")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Try TlsIdentity first, then X-Org-Id header.
        let org_id = {
            let ext = req.extensions();
            ext.get::<TlsIdentity>().map(|id| id.org_id.clone())
        }
        .or_else(|| {
            req.headers()
                .get("X-Org-Id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

        let start = std::time::Instant::now();
        let store = self.store.clone();
        let srv = self.service.clone();

        Box::pin(async move {
            let res = srv.call(req).await?;
            let duration_ms = start.elapsed().as_millis() as u64;
            let status_code = res.status().as_u16();

            let entry = crate::audit::AuditEntry {
                timestamp: chrono::Utc::now().to_rfc3339(),
                method,
                path,
                org_id,
                source_ip,
                status_code,
                trace_id,
                duration_ms,
            };

            // Fire-and-forget — never block the response for audit logging.
            if let Err(e) = store.append(&entry) {
                log::error!("audit log failed: {e}");
            }

            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Audit middleware tests ─────────────────────────────────────────────

    #[test]
    fn test_uuid_generation() {
        let uuid = Uuid::new_v4().to_string();
        assert!(!uuid.is_empty());
        assert_eq!(uuid.len(), 36); // Standard UUID v4 format
    }

    #[test]
    fn tls_identity_clone() {
        let id = TlsIdentity {
            org_id: "org1".to_string(),
            role: Some(crate::msp::MspRole::Admin),
        };
        let cloned = id.clone();
        assert_eq!(cloned.org_id, "org1");
        assert!(matches!(cloned.role, Some(crate::msp::MspRole::Admin)));
    }

    #[test]
    fn parse_x509_identity_extracts_cn_and_org() {
        // Build a minimal self-signed cert with CN=node1, O=rust-bc
        let cert_der = generate_test_cert("node1", "rust-bc");
        let id = parse_x509_identity(&cert_der).unwrap();
        assert_eq!(id.org_id, "rust-bc");
        assert!(matches!(id.role, Some(crate::msp::MspRole::Client)));
    }

    #[test]
    fn parse_x509_admin_role_inferred() {
        let cert_der = generate_test_cert("admin-user", "org1");
        let id = parse_x509_identity(&cert_der).unwrap();
        assert_eq!(id.org_id, "org1");
        assert!(matches!(id.role, Some(crate::msp::MspRole::Admin)));
    }

    #[test]
    fn parse_x509_peer_role_inferred() {
        let cert_der = generate_test_cert("peer0.org1", "org1");
        let id = parse_x509_identity(&cert_der).unwrap();
        assert!(matches!(id.role, Some(crate::msp::MspRole::Peer)));
    }

    /// Generate a self-signed X.509 cert DER for testing.
    fn generate_test_cert(cn: &str, org: &str) -> Vec<u8> {
        use rcgen::{CertificateParams, DistinguishedName, KeyPair};
        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, cn);
        dn.push(rcgen::DnType::OrganizationName, org);
        let mut params = CertificateParams::default();
        params.distinguished_name = dn;
        let key = KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        cert.der().to_vec()
    }
}
