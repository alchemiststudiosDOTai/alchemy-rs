pub mod error;
pub mod providers;
pub mod stream;
pub mod types;

pub use error::{Error, Result};
pub use providers::{get_env_api_key, stream_openai_completions, OpenAICompletionsOptions};
pub use stream::{complete, stream, AssistantMessageEventStream};
