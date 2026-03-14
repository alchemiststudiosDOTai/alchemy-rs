use crate::types::{GoogleGenerativeAi, InputType, KnownProvider, Model, ModelCost, Provider};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const ZERO_COST: ModelCost = ModelCost {
    input: 0.0,
    output: 0.0,
    cache_read: 0.0,
    cache_write: 0.0,
};

fn build(
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
        base_url: BASE_URL.to_string(),
        reasoning,
        input: vec![InputType::Text, InputType::Image],
        cost: ZERO_COST,
        context_window,
        max_tokens,
        headers: None,
        compat: None,
    }
}

pub fn gemini_2_5_pro() -> Model<GoogleGenerativeAi> {
    build("gemini-2.5-pro", "Gemini 2.5 Pro", true, 1_048_576, 65_536)
}

pub fn gemini_2_5_flash() -> Model<GoogleGenerativeAi> {
    build(
        "gemini-2.5-flash",
        "Gemini 2.5 Flash",
        true,
        1_048_576,
        65_536,
    )
}

pub fn gemini_2_5_flash_lite() -> Model<GoogleGenerativeAi> {
    build(
        "gemini-2.5-flash-lite",
        "Gemini 2.5 Flash Lite",
        false,
        1_048_576,
        65_536,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, ApiType};

    #[test]
    fn models_have_correct_api() {
        for model in [
            gemini_2_5_pro(),
            gemini_2_5_flash(),
            gemini_2_5_flash_lite(),
        ] {
            assert_eq!(model.api.api(), Api::GoogleGenerativeAi);
            assert_eq!(model.provider, Provider::Known(KnownProvider::Google));
            assert!(model.input.contains(&InputType::Image));
        }
    }

    #[test]
    fn reasoning_flags_correct() {
        assert!(gemini_2_5_pro().reasoning);
        assert!(gemini_2_5_flash().reasoning);
        assert!(!gemini_2_5_flash_lite().reasoning);
    }
}
