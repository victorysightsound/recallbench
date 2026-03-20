use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};

use crate::traits::LLMClient;

type Limiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Wraps any LLMClient with rate limiting.
///
/// Only applies to API mode — CLI providers handle their own rate limiting.
pub struct RateLimitedClient {
    inner: Box<dyn LLMClient>,
    limiter: Arc<Limiter>,
}

impl RateLimitedClient {
    /// Create a rate-limited wrapper.
    ///
    /// `rpm` is requests per minute.
    pub fn new(inner: Box<dyn LLMClient>, rpm: u32) -> Result<Self> {
        let rpm = NonZeroU32::new(rpm.max(1))
            .expect("rpm must be > 0");

        let limiter = Arc::new(RateLimiter::direct(
            Quota::per_minute(rpm),
        ));

        Ok(Self { inner, limiter })
    }
}

#[async_trait]
impl LLMClient for RateLimitedClient {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        self.limiter.until_ready().await;
        self.inner.generate(prompt, max_tokens).await
    }

    async fn generate_with_seed(&self, prompt: &str, max_tokens: usize, seed: u64) -> Result<String> {
        self.limiter.until_ready().await;
        self.inner.generate_with_seed(prompt, max_tokens, seed).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLLM;

    #[async_trait]
    impl LLMClient for MockLLM {
        fn name(&self) -> &str { "mock" }
        async fn generate(&self, _prompt: &str, _max_tokens: usize) -> Result<String> {
            Ok("response".to_string())
        }
    }

    #[tokio::test]
    async fn rate_limited_client_works() {
        let client = RateLimitedClient::new(Box::new(MockLLM), 60).unwrap();
        let result = client.generate("test", 100).await.unwrap();
        assert_eq!(result, "response");
        assert_eq!(client.name(), "mock");
    }
}
