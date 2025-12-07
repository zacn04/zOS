#[cfg(test)]
mod tests {
    use crate::circuit_breaker::{CircuitBreaker, ExponentialBackoff};
    use std::time::Duration;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new(60, 3);
        assert!(!cb.is_open());
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(60, 3);
        
        cb.record_failure();
        assert!(!cb.is_open());
        
        cb.record_failure();
        assert!(!cb.is_open());
        
        cb.record_failure();
        assert!(cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(60, 3);
        
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        
        assert!(!cb.is_open());
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_exponential_backoff() {
        let backoff = ExponentialBackoff::new(100, 5000);
        
        assert_eq!(backoff.delay_for_attempt(0), 100);
        assert_eq!(backoff.delay_for_attempt(1), 200);
        assert_eq!(backoff.delay_for_attempt(2), 400);
        assert_eq!(backoff.delay_for_attempt(3), 800);
        
        // Should cap at max
        assert!(backoff.delay_for_attempt(10) <= 5000);
    }
}
