# Line 801: test_rate_limiter_tokens_field_initialization
801 s/let mut limiter = RateLimiter::new(requests_per_minute);/let limiter = RateLimiter::new(requests_per_minute);/

# Line 813: test_rate_limiter_capacity_field
813 s/let mut limiter = RateLimiter::new(100.0);/let limiter = RateLimiter::new(100.0);/

# Line 826: test_rate_limiter_refill_rate_calculation
826 s/let mut limiter = RateLimiter::new(60.0);/let limiter = RateLimiter::new(60.0);/

# Line 838: test_rate_limiter_last_refill_timestamp
838 s/let mut limiter = RateLimiter::new(60.0);/let limiter = RateLimiter::new(60.0);/

# Line 852: test_rate_limiter_constructor
852 s/let mut limiter = RateLimiter::new(120.0);/let limiter = RateLimiter::new(120.0);/

# Line 1036: test_circuit_breaker_state_field_initialization
1036 s/let mut breaker = CircuitBreaker::new(5, 2, Duration::from_secs(60));/let breaker = CircuitBreaker::new(5, 2, Duration::from_secs(60));/

# Line 1077: test_circuit_breaker_constructor
1077 s/let mut breaker = CircuitBreaker::new(3, 2, Duration::from_secs(30));/let breaker = CircuitBreaker::new(3, 2, Duration::from_secs(30));/
