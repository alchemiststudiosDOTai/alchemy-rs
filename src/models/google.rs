use crate::types::{GoogleGenerativeAi, InputType, KnownProvider, Model, ModelCost, Provider};

const GOOGLE_AI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const ZERO_MODEL_COST: ModelCost = ModelCost {
    input: 0.0,
    output: 0.0,
    cache_read: 0.0,
    cache_write: 0.0,
};

fn build_google_model(
    id: &str,
    name: &str,
    reasoning: bool,
    context_window: u32,
    max_tokens: u32,
) -> Model<GoogleGenerativeAi> {
    Model {
        id: id.to_string(),
        name: name.to_string(),
        api: GoogleGenerativeAi,
        provider: Provider::Known(KnownProvider::Google),
        base_url: GOOGLE_AI_BASE_URL.to_string(),
        reasoning,
        input: vec![InputType::Text, InputType::Image],
        cost: ZERO_MODEL_COST,
        context_window,
        max_tokens,
        headers: None,
        compat: None,
    }
}

pub fn gemini_2_5_pro() -> Model<GoogleGenerativeAi> {
    build_google_model(
        "gemini-2.5-pro-latest",
        "Gemini 2.5 Pro",
        true,
        1_048_576,
        65_536,
    )
}

pub fn gemini_2_5_flash() -> Model<GoogleGenerativeAi> {
    build_google_model(
        "gemini-2.5-flash-latest",
        "Gemini 2.5 Flash",
        true,
        1_048_576,
        65_536,
    )
}

pub fn gemini_2_5_flash_lite() -> Model<GoogleGenerativeAi> {
    build_google_model(
        "gemini-2.5-flash-lite-latest",
        "Gemini 2.5 Flash Lite",
        false,
        1_048_576,
        65_536,
    )
}

pub fn gemini_2_0_flash() -> Model<GoogleGenerativeAi> {
    build_google_model(
        "gemini-2.0-flash",
        "Gemini 2.0 Flash",
        false,
        1_048_576,
        8_192,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, ApiType};

    #[test]
    fn gemini_2_5_pro_model_has_correct_api_and_provider() {
        let model = gemini_2_5_pro();
        assert_eq!(model.api.api(), Api::GoogleGenerativeAi);
        assert_eq!(model.provider, Provider::Known(KnownProvider::Google));
        assert_eq!(model.base_url, GOOGLE_AI_BASE_URL);
        assert!(model.reasoning);
    }

    #[test]
    fn gemini_2_0_flash_model_is_non_reasoning() {
        let model = gemini_2_0_flash();
        assert_eq!(model.api.api(), Api::GoogleGenerativeAi);
        assert!(!model.reasoning);
        assert_eq!(model.max_tokens, 8_192);
    }

    #[test]
    fn all_google_models_support_image_input() {
        for model in [
            gemini_2_5_pro(),
            gemini_2_5_flash(),
            gemini_2_5_flash_lite(),
            gemini_2_0_flash(),
        ] {
            assert!(
                model.input.contains(&InputType::Image),
                "{} should support image input",
                model.name
            );
        }
    }
}
