use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use std::{
    collections::HashMap,
    future::{ready, Future, Ready},
    pin::Pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use std::task::{Context, Poll};

/**
 * Configuración de rate limiting
 */
#[derive(Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            requests_per_minute: 100,
            requests_per_hour: 1000,
        }
    }
}

/**
 * Información de rate limiting por IP
 */
struct RateLimitInfo {
    minute_requests: Vec<Instant>,
    hour_requests: Vec<Instant>,
}

impl RateLimitInfo {
    fn new() -> Self {
        RateLimitInfo {
            minute_requests: Vec::new(),
            hour_requests: Vec::new(),
        }
    }

    fn cleanup_old_requests(&mut self) {
        let now = Instant::now();
        let minute_ago = now - Duration::from_secs(60);
        let hour_ago = now - Duration::from_secs(3600);

        self.minute_requests.retain(|&time| time > minute_ago);
        self.hour_requests.retain(|&time| time > hour_ago);
    }

    fn check_limit(&mut self, config: &RateLimitConfig) -> bool {
        self.cleanup_old_requests();

        let now = Instant::now();
        
        if self.minute_requests.len() >= config.requests_per_minute as usize {
            return false;
        }

        if self.hour_requests.len() >= config.requests_per_hour as usize {
            return false;
        }

        self.minute_requests.push(now);
        self.hour_requests.push(now);
        true
    }
}

/**
 * Middleware de rate limiting
 */
pub struct RateLimitMiddleware {
    config: RateLimitConfig,
    limits: Arc<Mutex<HashMap<String, RateLimitInfo>>>,
}

impl RateLimitMiddleware {
    /**
     * Crea un nuevo middleware de rate limiting
     * @param config - Configuración de límites
     * @returns RateLimitMiddleware configurado
     */
    pub fn new(config: RateLimitConfig) -> Self {
        RateLimitMiddleware {
            config,
            limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /**
     * Obtiene la IP del cliente desde la request
     */
    fn get_client_ip(req: &ServiceRequest) -> String {
        if let Some(peer_addr) = req.peer_addr() {
            return peer_addr.ip().to_string();
        }
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(ip) = forwarded_str.split(',').next() {
                    return ip.trim().to_string();
                }
            }
        }
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                return ip_str.to_string();
            }
        }
        "127.0.0.1".to_string()
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitService {
            service,
            config: self.config.clone(),
            limits: self.limits.clone(),
        }))
    }
}

pub struct RateLimitService<S> {
    service: S,
    config: RateLimitConfig,
    limits: Arc<Mutex<HashMap<String, RateLimitInfo>>>,
}

impl<S, B> Service<ServiceRequest> for RateLimitService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let ip = RateLimitMiddleware::get_client_ip(&req);
        let limits = self.limits.clone();
        let config = self.config.clone();
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut limits_guard = limits.lock().unwrap_or_else(|e| e.into_inner());
            let rate_limit_info = limits_guard.entry(ip.clone()).or_insert_with(RateLimitInfo::new);

            if !rate_limit_info.check_limit(&config) {
                drop(limits_guard);
                return Err(actix_web::error::ErrorTooManyRequests("Rate limit exceeded"));
            }

            drop(limits_guard);
            fut.await
        })
    }
}

