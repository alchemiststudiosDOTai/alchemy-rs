//! Shared utilities for provider implementations.

mod http;
mod openai_like_messages;
mod openai_like_runtime;
mod stream_blocks;
mod timestamp;

pub(crate) use http::build_http_client;
pub(crate) use openai_like_messages::{
    convert_messages, convert_tools, AssistantThinkingMode, OpenAiLikeMessageOptions,
    SystemPromptRole,
};
pub(crate) use openai_like_runtime::{
    initialize_output, process_sse_stream, push_stream_done, push_stream_error,
    send_streaming_request,
};
pub(crate) use stream_blocks::{
    finish_current_block, handle_reasoning_delta, handle_text_delta, handle_tool_calls,
    map_stop_reason, update_usage_from_chunk, CurrentBlock, OpenAiLikeStreamUsage,
    OpenAiLikeToolCallDelta, ReasoningDelta,
};
