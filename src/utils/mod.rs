//! Cross-provider utilities for consistent behavior across all LLM providers.
//!
//! This module provides utilities for:
//! - Unicode sanitization for API requests
//! - Partial JSON parsing for streaming tool calls

pub mod json_parse;
pub mod sanitize;

pub use json_parse::{parse_streaming_json, parse_streaming_json_smart};
pub use sanitize::{sanitize_for_api, sanitize_surrogates};
