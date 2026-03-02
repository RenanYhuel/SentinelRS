use anyhow::Result;
use futures_util::StreamExt;

pub struct SseEvent {
    pub event: String,
    pub data: String,
}

pub async fn stream_events(
    client: &reqwest::Client,
    url: &str,
    mut handler: impl FnMut(SseEvent) -> bool,
) -> Result<()> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut current_event = String::new();
    let mut current_data = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(pos) = buffer.find("\n\n") {
            let block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in block.lines() {
                if let Some(val) = line.strip_prefix("event:") {
                    current_event = val.trim().to_string();
                } else if let Some(val) = line.strip_prefix("data:") {
                    current_data = val.trim().to_string();
                }
            }

            if !current_data.is_empty() {
                let should_continue = handler(SseEvent {
                    event: std::mem::take(&mut current_event),
                    data: std::mem::take(&mut current_data),
                });
                if !should_continue {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}
