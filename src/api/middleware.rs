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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_generation() {
        let uuid = Uuid::new_v4().to_string();
        assert!(!uuid.is_empty());
        assert_eq!(uuid.len(), 36); // Standard UUID v4 format
    }
}
