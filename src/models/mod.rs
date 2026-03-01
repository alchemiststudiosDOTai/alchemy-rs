pub mod google;
pub mod minimax;

pub use google::{gemini_2_0_flash, gemini_2_5_flash, gemini_2_5_flash_lite, gemini_2_5_pro};
pub use minimax::{
    minimax_cn_m2, minimax_cn_m2_1, minimax_cn_m2_1_highspeed, minimax_cn_m2_5,
    minimax_cn_m2_5_highspeed, minimax_m2, minimax_m2_1, minimax_m2_1_highspeed, minimax_m2_5,
    minimax_m2_5_highspeed,
};
