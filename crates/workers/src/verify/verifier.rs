use sentinel_common::canonicalize::canonical_bytes;
use sentinel_common::crypto::verify_signature;
use sentinel_common::proto::Batch;

use super::secret_provider::SecretProvider;

pub enum VerifyResult {
    Valid,
    Invalid,
    Skipped,
}

pub async fn verify_batch(
    provider: &dyn SecretProvider,
    batch: &Batch,
    signature: Option<&str>,
) -> VerifyResult {
    let Some(sig) = signature else {
        tracing::warn!(agent_id = %batch.agent_id, "no signature header, skipping verify");
        return VerifyResult::Skipped;
    };

    let Some(secret) = provider.get_secret(&batch.agent_id).await else {
        tracing::warn!(agent_id = %batch.agent_id, "no secret available, skipping verify");
        return VerifyResult::Skipped;
    };

    let canonical = canonical_bytes(batch);
    if verify_signature(&secret, &canonical, sig) {
        VerifyResult::Valid
    } else {
        VerifyResult::Invalid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_common::crypto::sign_data;
    use std::collections::HashMap;

    struct StaticProvider {
        secrets: HashMap<String, Vec<u8>>,
    }

    #[tonic::async_trait]
    impl SecretProvider for StaticProvider {
        async fn get_secret(&self, agent_id: &str) -> Option<Vec<u8>> {
            self.secrets.get(agent_id).cloned()
        }
    }

    fn test_batch() -> Batch {
        Batch {
            agent_id: "agent-1".into(),
            batch_id: "b-1".into(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn valid_signature() {
        let batch = test_batch();
        let secret = b"test-secret".to_vec();
        let canonical = canonical_bytes(&batch);
        let sig = sign_data(&secret, &canonical);
        let provider = StaticProvider {
            secrets: HashMap::from([("agent-1".into(), secret)]),
        };
        match verify_batch(&provider, &batch, Some(&sig)).await {
            VerifyResult::Valid => {}
            _ => panic!("expected Valid"),
        }
    }

    #[tokio::test]
    async fn invalid_signature() {
        let batch = test_batch();
        let provider = StaticProvider {
            secrets: HashMap::from([("agent-1".into(), b"secret".to_vec())]),
        };
        match verify_batch(&provider, &batch, Some("bad-sig")).await {
            VerifyResult::Invalid => {}
            _ => panic!("expected Invalid"),
        }
    }

    #[tokio::test]
    async fn no_signature_skips() {
        let batch = test_batch();
        let provider = StaticProvider {
            secrets: HashMap::new(),
        };
        match verify_batch(&provider, &batch, None).await {
            VerifyResult::Skipped => {}
            _ => panic!("expected Skipped"),
        }
    }

    #[tokio::test]
    async fn no_secret_skips() {
        let batch = test_batch();
        let provider = StaticProvider {
            secrets: HashMap::new(),
        };
        match verify_batch(&provider, &batch, Some("some-sig")).await {
            VerifyResult::Skipped => {}
            _ => panic!("expected Skipped"),
        }
    }
}
