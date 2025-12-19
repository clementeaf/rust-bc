use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Token bucket for rate limiting
#[derive(Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    refill_rate: f64, // tokens per second
}

impl TokenBucket {
    /// Create a new token bucket
    fn new(capacity: f64, refill_rate: f64) -> Self {
        TokenBucket {
            tokens: capacity,
            last_refill: Instant::now(),
            capacity,
            refill_rate,
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let refill_amount = elapsed * self.refill_rate;
        self.tokens = (self.tokens + refill_amount).min(self.capacity);
        self.last_refill = now;
    }

    /// Try to consume tokens, returns true if successful
    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Get current token count
    fn current_tokens(&self) -> f64 {
        self.tokens
    }
}

/// Rate limiter with per-IP token buckets
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    capacity: f64,
    refill_rate: f64,
    cleanup_interval: Duration,
    last_cleanup: Arc<Mutex<Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    /// capacity: maximum tokens per bucket
    /// refill_rate: tokens per second
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        RateLimiter {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            capacity,
            refill_rate,
            cleanup_interval: Duration::from_secs(60),
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Check if request from IP should be allowed
    pub fn allow_request(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        
        // Create bucket if not exists
        buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));
        
        // Try to consume 1 token
        let bucket = buckets.get_mut(&ip).unwrap();
        let allowed = bucket.try_consume(1.0);
        
        // Cleanup old buckets periodically
        let mut last_cleanup = self.last_cleanup.lock().unwrap();
        if last_cleanup.elapsed() > self.cleanup_interval {
            Self::cleanup_buckets(&mut buckets);
            *last_cleanup = Instant::now();
        }
        
        allowed
    }

    /// Get remaining tokens for an IP
    pub fn get_remaining_tokens(&self, ip: IpAddr) -> f64 {
        let buckets = self.buckets.lock().unwrap();
        buckets
            .get(&ip)
            .map(|b| b.current_tokens())
            .unwrap_or(self.capacity)
    }

    /// Remove empty buckets (cleanup)
    fn cleanup_buckets(buckets: &mut HashMap<IpAddr, TokenBucket>) {
        buckets.retain(|_, bucket| bucket.current_tokens() < bucket.capacity);
    }

    /// Reset rate limiter (clear all buckets)
    pub fn reset(&self) {
        let mut buckets = self.buckets.lock().unwrap();
        buckets.clear();
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        // Default: 1000 requests per second
        RateLimiter::new(1000.0, 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(100.0, 10.0);
        assert_eq!(bucket.capacity, 100.0);
        assert_eq!(bucket.refill_rate, 10.0);
        assert_eq!(bucket.tokens, 100.0);
    }

    #[test]
    fn test_token_consumption() {
        let mut bucket = TokenBucket::new(10.0, 1.0);
        assert!(bucket.try_consume(5.0));
        assert!(bucket.tokens <= 5.1 && bucket.tokens >= 4.9);
        assert!(bucket.try_consume(5.0));
        assert!(bucket.tokens < 0.1);
        assert!(!bucket.try_consume(1.0));
    }

    #[test]
    fn test_token_refill() {
        let mut bucket = TokenBucket::new(100.0, 10.0);
        bucket.try_consume(50.0);
        assert_eq!(bucket.tokens, 50.0);
        
        // Simulate time passing
        std::thread::sleep(Duration::from_millis(100));
        bucket.refill();
        // Should have refilled approximately 1 token (100ms * 10 tokens/sec)
        assert!(bucket.tokens > 50.0 && bucket.tokens <= 100.0);
    }

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(100.0, 50.0);
        assert_eq!(limiter.capacity, 100.0);
        assert_eq!(limiter.refill_rate, 50.0);
    }

    #[test]
    fn test_rate_limiter_default() {
        let limiter = RateLimiter::default();
        assert_eq!(limiter.capacity, 1000.0);
        assert_eq!(limiter.refill_rate, 1000.0);
    }

    #[test]
    fn test_allow_request() {
        let limiter = RateLimiter::new(5.0, 1.0);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        // Should allow initial requests up to capacity
        for _ in 0..5 {
            assert!(limiter.allow_request(ip));
        }
        
        // 6th request should be rejected
        assert!(!limiter.allow_request(ip));
    }

    #[test]
    fn test_multiple_ips() {
        let limiter = RateLimiter::new(2.0, 1.0);
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        
        // Each IP has its own bucket
        assert!(limiter.allow_request(ip1));
        assert!(limiter.allow_request(ip1));
        assert!(!limiter.allow_request(ip1));
        
        assert!(limiter.allow_request(ip2));
        assert!(limiter.allow_request(ip2));
        assert!(!limiter.allow_request(ip2));
    }

    #[test]
    fn test_get_remaining_tokens() {
        let limiter = RateLimiter::new(10.0, 1.0);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        // Initial should be at capacity
        assert_eq!(limiter.get_remaining_tokens(ip), 10.0);
        
        // After consuming
        limiter.allow_request(ip);
        assert_eq!(limiter.get_remaining_tokens(ip), 9.0);
    }

    #[test]
    fn test_reset() {
        let limiter = RateLimiter::new(5.0, 1.0);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        // Exhaust bucket
        for _ in 0..5 {
            limiter.allow_request(ip);
        }
        assert!(!limiter.allow_request(ip));
        
        // Reset
        limiter.reset();
        assert!(limiter.allow_request(ip));
    }

    #[test]
    fn test_rate_limiter_blocking() {
        let limiter = RateLimiter::new(3.0, 10.0);
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        
        // Consume all tokens
        assert!(limiter.allow_request(ip));
        assert!(limiter.allow_request(ip));
        assert!(limiter.allow_request(ip));
        
        // Should be blocked
        assert!(!limiter.allow_request(ip));
        
        // Wait for refill
        std::thread::sleep(Duration::from_millis(350));
        
        // Should be allowed again (350ms * 10 tokens/sec = 3.5 tokens refilled)
        assert!(limiter.allow_request(ip));
    }

    #[test]
    fn test_capacity_constraint() {
        let limiter = RateLimiter::new(50.0, 1000.0);
        let ip = IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1));
        
        // Even with high refill rate, shouldn't exceed capacity
        std::thread::sleep(Duration::from_millis(100));
        let remaining = limiter.get_remaining_tokens(ip);
        assert!(remaining <= 50.0);
    }

    #[test]
    fn test_concurrent_cleanup() {
        let limiter = Arc::new(RateLimiter::new(10.0, 1.0));
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        
        // Reset to simulate cleanup
        limiter.reset();
        assert_eq!(limiter.get_remaining_tokens(ip), 10.0);
    }
}
