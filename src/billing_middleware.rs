use crate::billing::BillingManager;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

/**
 * Middleware para validar API keys y aplicar límites de billing
 */
pub struct BillingMiddleware {
    billing_manager: Arc<BillingManager>,
}

impl BillingMiddleware {
    /**
     * Crea un nuevo middleware de billing
     */
    pub fn new(billing_manager: Arc<BillingManager>) -> BillingMiddleware {
        BillingMiddleware { billing_manager }
    }
}

impl<S, B> Transform<S, ServiceRequest> for BillingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = BillingService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(BillingService {
            service: Rc::new(service),
            billing_manager: Arc::clone(&self.billing_manager),
        }))
    }
}

/**
 * Servicio que aplica validación de billing
 */
pub struct BillingService<S> {
    service: Rc<S>,
    billing_manager: Arc<BillingManager>,
}

impl<S, B> Service<ServiceRequest> for BillingService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let billing_manager = Arc::clone(&self.billing_manager);
        let path = req.path().to_string();

        Box::pin(async move {
            let api_key = Self::extract_api_key(&req);
            
            if Self::requires_auth(&path) {
                if api_key.is_none() {
                    return Err(actix_web::error::ErrorUnauthorized(
                        "API key requerida. Incluye 'X-API-Key' en el header.",
                    ));
                }

                let key = api_key.unwrap();
                
                match billing_manager.validate_key(&key) {
                    Ok(key_info) => {
                        if !key_info.is_active {
                            return Err(actix_web::error::ErrorForbidden(
                                "API key desactivada",
                            ));
                        }

                        if let Err(e) = billing_manager.record_request(&key) {
                            return Err(actix_web::error::ErrorInternalServerError(e));
                        }

                        req.extensions_mut().insert(key_info);
                    }
                    Err(e) => {
                        return Err(actix_web::error::ErrorUnauthorized(e));
                    }
                }
            }

            let fut = service.call(req);
            fut.await
        })
    }
}

impl<S> BillingService<S> {
    /**
     * Extrae la API key del header de la request
     */
    fn extract_api_key(req: &ServiceRequest) -> Option<String> {
        req.headers()
            .get("X-API-Key")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }

    /**
     * Verifica si una ruta requiere autenticación
     */
    fn requires_auth(path: &str) -> bool {
        let public_paths = [
            "/api/v1/health",
            "/api/v1/billing/create-key",
        ];
        
        !public_paths.iter().any(|p| path.starts_with(p))
    }
}

