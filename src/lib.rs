pub mod error;
pub mod providers;
pub mod stream;
pub mod transform;
pub mod types;
pub mod utils;

pub use error::{Error, Result};
pub use providers::{get_env_api_key, stream_openai_completions, OpenAICompletionsOptions};
pub use stream::{complete, stream, AssistantMessageEventStream};
pub use transform::{transform_messages, transform_messages_simple, TargetModel};
pub use types::{is_context_overflow, validate_tool_arguments, validate_tool_call};
pub use utils::{
    parse_streaming_json, parse_streaming_json_smart, sanitize_for_api, sanitize_surrogates,
};
