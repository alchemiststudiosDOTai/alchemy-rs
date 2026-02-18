use futures::StreamExt;
use serde::de::DeserializeOwned;

use crate::stream::EventStreamSender;
use crate::types::{
    Api, AssistantMessage, AssistantMessageEvent, Provider, StopReason, StopReasonError,
    StopReasonSuccess, Usage,
};

use super::timestamp::unix_timestamp_millis;

pub(crate) fn initialize_output(api: Api, provider: Provider, model: String) -> AssistantMessage {
    AssistantMessage {
        content: vec![],
        api,
        provider,
        model,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp: unix_timestamp_millis(),
    }
}

pub(crate) fn push_stream_error(
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    error: crate::Error,
) {
    output.stop_reason = StopReason::Error;
    output.error_message = Some(error.to_string());

    sender.push(AssistantMessageEvent::Error {
        reason: StopReasonError::Error,
        error: output.clone(),
    });
}

pub(crate) fn push_stream_done(output: &AssistantMessage, sender: &mut EventStreamSender) {
    sender.push(AssistantMessageEvent::Done {
        reason: done_reason_from_stop_reason(output.stop_reason),
        message: output.clone(),
    });
}

fn done_reason_from_stop_reason(stop_reason: StopReason) -> StopReasonSuccess {
    match stop_reason {
        StopReason::Stop => StopReasonSuccess::Stop,
        StopReason::Length => StopReasonSuccess::Length,
        StopReason::ToolUse => StopReasonSuccess::ToolUse,
        _ => StopReasonSuccess::Stop,
    }
}

pub(crate) async fn send_streaming_request(
    client: &reqwest::Client,
    base_url: &str,
    params: &serde_json::Value,
) -> Result<reqwest::Response, crate::Error> {
    let response = client.post(base_url).json(params).send().await?;

    if response.status().is_success() {
        return Ok(response);
    }

    let status_code = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();

    Err(crate::Error::ApiError {
        status_code,
        message: body,
    })
}

pub(crate) async fn process_sse_stream<TChunk, F>(
    response: reqwest::Response,
    mut on_chunk: F,
) -> Result<(), crate::Error>
where
    TChunk: DeserializeOwned,
    F: FnMut(TChunk),
{
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut done_received = false;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    done_received = true;
                    break;
                }

                if let Ok(chunk) = serde_json::from_str::<TChunk>(data) {
                    on_chunk(chunk);
                }
            }
        }

        if done_received {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::done_reason_from_stop_reason;
    use crate::types::{StopReason, StopReasonSuccess};

    #[test]
    fn stop_reason_maps_to_done_reason() {
        assert_eq!(
            done_reason_from_stop_reason(StopReason::Stop),
            StopReasonSuccess::Stop
        );
        assert_eq!(
            done_reason_from_stop_reason(StopReason::Length),
            StopReasonSuccess::Length
        );
        assert_eq!(
            done_reason_from_stop_reason(StopReason::ToolUse),
            StopReasonSuccess::ToolUse
        );
        assert_eq!(
            done_reason_from_stop_reason(StopReason::Error),
            StopReasonSuccess::Stop
        );
    }
}
