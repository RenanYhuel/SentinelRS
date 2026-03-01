use std::future::Future;
use std::time::Duration;

pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            backoff_factor: 2.0,
        }
    }
}

pub async fn retry_async<F, Fut, T, E>(config: &RetryConfig, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut delay = config.initial_delay;
    let mut last_err = None;

    for attempt in 1..=config.max_attempts {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                eprintln!(
                    "[retry] attempt {attempt}/{} failed: {e:?}",
                    config.max_attempts
                );
                last_err = Some(e);
                if attempt < config.max_attempts {
                    tokio::time::sleep(delay).await;
                    delay = Duration::from_secs_f64(delay.as_secs_f64() * config.backoff_factor);
                }
            }
        }
    }

    Err(last_err.unwrap())
}

pub fn retry_sync<F, T, E>(config: &RetryConfig, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Debug,
{
    let mut delay = config.initial_delay;
    let mut last_err = None;

    for attempt in 1..=config.max_attempts {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                eprintln!(
                    "[retry] attempt {attempt}/{} failed: {e:?}",
                    config.max_attempts
                );
                last_err = Some(e);
                if attempt < config.max_attempts {
                    std::thread::sleep(delay);
                    delay = Duration::from_secs_f64(delay.as_secs_f64() * config.backoff_factor);
                }
            }
        }
    }

    Err(last_err.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn succeeds_on_first_try() {
        let config = RetryConfig::default();
        let result = retry_async(&config, || async { Ok::<_, &str>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn succeeds_after_retries() {
        let counter = AtomicU32::new(0);
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            backoff_factor: 1.0,
        };

        let result: Result<u32, &str> = retry_async(&config, || {
            let attempt = counter.fetch_add(1, Ordering::SeqCst) + 1;
            async move {
                if attempt < 3 {
                    Err("not yet")
                } else {
                    Ok(attempt)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 3);
    }

    #[tokio::test]
    async fn fails_after_max_attempts() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            backoff_factor: 1.0,
        };

        let result: Result<(), &str> = retry_async(&config, || async { Err("always fails") }).await;

        assert!(result.is_err());
    }

    #[test]
    fn sync_retry_succeeds() {
        let config = RetryConfig::default();
        let result = retry_sync(&config, || Ok::<_, &str>(10));
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn sync_retry_exhausts() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            backoff_factor: 1.0,
        };
        let result: Result<(), &str> = retry_sync(&config, || Err("nope"));
        assert!(result.is_err());
    }
}
