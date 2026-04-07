use crate::types::{GoogleGenerativeAi, InputType, KnownProvider, Model, ModelCost, Provider};

const GOOGLE_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const GOOGLE_CONTEXT_WINDOW: u32 = 1_048_576;
const GOOGLE_MAX_OUTPUT_TOKENS: u32 = 65_536;
const ZERO_MODEL_COST: ModelCost = ModelCost {
    input: 0.0,
    output: 0.0,
    cache_read: 0.0,
    cache_write: 0.0,
};

fn build_google_model(id: &str, name: &str) -> Model<GoogleGenerativeAi> {
    Model {
        id: id.to_string(),
        name: name.to_string(),
        api: GoogleGenerativeAi,
        provider: Provider::Known(KnownProvider::Google),
        base_url: GOOGLE_BASE_URL.to_string(),
        reasoning: true,
        input: vec![InputType::Text, InputType::Image],
        cost: ZERO_MODEL_COST,
        context_window: GOOGLE_CONTEXT_WINDOW,
        max_tokens: GOOGLE_MAX_OUTPUT_TOKENS,
        headers: None,
        compat: None,
    }
}

pub fn gemini_2_5_pro() -> Model<GoogleGenerativeAi> {
    build_google_model("gemini-2.5-pro", "Gemini 2.5 Pro")
}

pub fn gemini_2_5_flash() -> Model<GoogleGenerativeAi> {
    build_google_model("gemini-2.5-flash", "Gemini 2.5 Flash")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, ApiType};

    #[test]
    fn gemini_model_has_google_api() {
        let model = gemini_2_5_pro();
        assert_eq!(model.api.api(), Api::GoogleGenerativeAi);
        assert_eq!(model.provider, Provider::Known(KnownProvider::Google));
        assert_eq!(model.base_url, GOOGLE_BASE_URL);
        assert!(model.reasoning);
    }

    #[test]
    fn gemini_flash_model_supports_images() {
        let model = gemini_2_5_flash();
        assert!(model.input.contains(&InputType::Image));
        assert!(model.reasoning);
    }
}
