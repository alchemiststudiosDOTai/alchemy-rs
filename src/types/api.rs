use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Api {
    AnthropicMessages,
    BedrockConverseStream,
    OpenAICompletions,
    OpenAIResponses,
    GoogleGenerativeAi,
    GoogleVertex,
}

impl Display for Api {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AnthropicMessages => write!(f, "anthropic-messages"),
            Self::BedrockConverseStream => write!(f, "bedrock-converse-stream"),
            Self::OpenAICompletions => write!(f, "openai-completions"),
            Self::OpenAIResponses => write!(f, "openai-responses"),
            Self::GoogleGenerativeAi => write!(f, "google-generative-ai"),
            Self::GoogleVertex => write!(f, "google-vertex"),
        }
    }
}

impl FromStr for Api {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "anthropic-messages" => Ok(Self::AnthropicMessages),
            "bedrock-converse-stream" => Ok(Self::BedrockConverseStream),
            "openai-completions" => Ok(Self::OpenAICompletions),
            "openai-responses" => Ok(Self::OpenAIResponses),
            "google-generative-ai" => Ok(Self::GoogleGenerativeAi),
            "google-vertex" => Ok(Self::GoogleVertex),
            _ => Err(crate::Error::UnknownApi(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum KnownProvider {
    AmazonBedrock,
    Anthropic,
    Google,
    GoogleVertex,
    OpenAI,
    Xai,
    Groq,
    Cerebras,
    OpenRouter,
    VercelAiGateway,
    Zai,
    Mistral,
    Minimax,
    MinimaxCn,
}

impl Display for KnownProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AmazonBedrock => write!(f, "amazon-bedrock"),
            Self::Anthropic => write!(f, "anthropic"),
            Self::Google => write!(f, "google"),
            Self::GoogleVertex => write!(f, "google-vertex"),
            Self::OpenAI => write!(f, "openai"),
            Self::Xai => write!(f, "xai"),
            Self::Groq => write!(f, "groq"),
            Self::Cerebras => write!(f, "cerebras"),
            Self::OpenRouter => write!(f, "openrouter"),
            Self::VercelAiGateway => write!(f, "vercel-ai-gateway"),
            Self::Zai => write!(f, "zai"),
            Self::Mistral => write!(f, "mistral"),
            Self::Minimax => write!(f, "minimax"),
            Self::MinimaxCn => write!(f, "minimax-cn"),
        }
    }
}

impl FromStr for KnownProvider {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amazon-bedrock" => Ok(Self::AmazonBedrock),
            "anthropic" => Ok(Self::Anthropic),
            "google" => Ok(Self::Google),
            "google-vertex" => Ok(Self::GoogleVertex),
            "openai" => Ok(Self::OpenAI),
            "xai" => Ok(Self::Xai),
            "groq" => Ok(Self::Groq),
            "cerebras" => Ok(Self::Cerebras),
            "openrouter" => Ok(Self::OpenRouter),
            "vercel-ai-gateway" => Ok(Self::VercelAiGateway),
            "zai" => Ok(Self::Zai),
            "mistral" => Ok(Self::Mistral),
            "minimax" => Ok(Self::Minimax),
            "minimax-cn" => Ok(Self::MinimaxCn),
            _ => Err(crate::Error::UnknownProvider(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Provider {
    Known(KnownProvider),
    Custom(String),
}

impl Provider {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(k) => match k {
                KnownProvider::AmazonBedrock => "amazon-bedrock",
                KnownProvider::Anthropic => "anthropic",
                KnownProvider::Google => "google",
                KnownProvider::GoogleVertex => "google-vertex",
                KnownProvider::OpenAI => "openai",
                KnownProvider::Xai => "xai",
                KnownProvider::Groq => "groq",
                KnownProvider::Cerebras => "cerebras",
                KnownProvider::OpenRouter => "openrouter",
                KnownProvider::VercelAiGateway => "vercel-ai-gateway",
                KnownProvider::Zai => "zai",
                KnownProvider::Mistral => "mistral",
                KnownProvider::Minimax => "minimax",
                KnownProvider::MinimaxCn => "minimax-cn",
            },
            Self::Custom(s) => s,
        }
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Provider {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match KnownProvider::from_str(s) {
            Ok(k) => Ok(Self::Known(k)),
            Err(_) => Ok(Self::Custom(s.to_string())),
        }
    }
}

impl From<KnownProvider> for Provider {
    fn from(value: KnownProvider) -> Self {
        Self::Known(value)
    }
}

pub trait ApiType: Send + Sync {
    type Compat: CompatibilityOptions;
    fn api(&self) -> Api;
}

pub trait CompatibilityOptions: Send + Sync + 'static {
    fn as_any(&self) -> Option<&dyn std::any::Any>;
}

#[derive(Debug, Clone, Copy)]
pub struct NoCompat;

impl CompatibilityOptions for NoCompat {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}
