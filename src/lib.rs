pub mod error;
pub mod models;
pub mod providers;
pub mod stream;
pub mod transform;
pub mod types;
pub mod utils;

pub use error::{Error, Result};
pub use models::{
    minimax_cn_m2, minimax_cn_m2_1, minimax_cn_m2_1_highspeed, minimax_cn_m2_5,
    minimax_cn_m2_5_highspeed, minimax_m2, minimax_m2_1, minimax_m2_1_highspeed, minimax_m2_5,
    minimax_m2_5_highspeed,
};
pub use providers::{
    get_env_api_key, stream_minimax_completions, stream_openai_completions,
    OpenAICompletionsOptions,
};
pub use stream::{complete, stream, AssistantMessageEventStream};
pub use transform::{transform_messages, transform_messages_simple, TargetModel};
pub use utils::{
    is_context_overflow, parse_streaming_json, parse_streaming_json_smart, sanitize_for_api,
    sanitize_surrogates, validate_tool_arguments, validate_tool_call, ThinkFragment,
    ThinkTagParser,
};
