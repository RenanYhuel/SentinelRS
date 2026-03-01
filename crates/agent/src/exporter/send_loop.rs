use super::client::GrpcClient;
use super::retry::RetryPolicy;
use crate::batch::BatchComposer;
use crate::buffer::Wal;
use sentinel_common::proto::push_response::Status;

pub struct SendLoop {
    pub retry_policy: RetryPolicy,
}

impl SendLoop {
    pub async fn send_pending(
        &self,
        wal: &mut Wal,
        client: &mut GrpcClient,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let unacked = wal.iter_unacked()?;
        let mut sent_count = 0;

        for (record_id, data) in unacked {
            let batch = BatchComposer::decode_batch(&data)?;
            let mut attempt = 0;

            loop {
                match client.push_metrics(batch.clone()).await {
                    Ok(resp) => match Status::try_from(resp.status) {
                        Ok(Status::Ok) => {
                            wal.ack(record_id);
                            sent_count += 1;
                            break;
                        }
                        Ok(Status::Rejected) => {
                            wal.ack(record_id);
                            break;
                        }
                        Ok(Status::Retry) | Err(_) => {
                            if !self.retry_policy.should_retry(attempt) {
                                break;
                            }
                            tokio::time::sleep(self.retry_policy.delay_for_attempt(attempt)).await;
                            attempt += 1;
                        }
                    },
                    Err(_) => {
                        if !self.retry_policy.should_retry(attempt) {
                            break;
                        }
                        tokio::time::sleep(self.retry_policy.delay_for_attempt(attempt)).await;
                        attempt += 1;
                    }
                }
            }
        }

        wal.save_meta()?;
        Ok(sent_count)
    }
}
