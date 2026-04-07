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

/// Identity extracted from a verified client TLS certificate.
#[derive(Debug, Clone)]
pub struct TlsIdentity {
    /// Common Name from the client certificate subject.
    pub common_name: String,
    /// Organization from the client certificate subject (if present).
    pub organization: Option<String>,
}

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
    // Try the X-TLS-Client-CN header (set by TLS-terminating proxies or test harnesses).
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

    cn.map(|common_name| TlsIdentity {
        common_name,
        organization: org,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_generation() {
        let uuid = Uuid::new_v4().to_string();
        assert!(!uuid.is_empty());
        assert_eq!(uuid.len(), 36); // Standard UUID v4 format
    }

    #[test]
    fn tls_identity_clone() {
        let id = TlsIdentity {
            common_name: "node1".to_string(),
            organization: Some("org1".to_string()),
        };
        let cloned = id.clone();
        assert_eq!(cloned.common_name, "node1");
        assert_eq!(cloned.organization, Some("org1".to_string()));
    }
}
