#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No API key provided for provider: {0}")]
    NoApiKey(String),

    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("API returned error: {status_code} - {message}")]
    ApiError { status_code: u16, message: String },

    #[error("Stream aborted")]
    Aborted,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Model not found: provider={provider}, model_id={model_id}")]
    ModelNotFound { provider: String, model_id: String },

    #[error("Unknown provider: {0}")]
    UnknownProvider(String),

    #[error("Unknown API: {0}")]
    UnknownApi(String),
}

pub type Result<T> = std::result::Result<T, Error>;
