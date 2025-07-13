pub mod formatter;

pub use formatter::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct PixelatedImage {
    pub lines: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RatatuiImage {
    pub pixels: Vec<Vec<(u8, u8, u8)>>, // RGB values for each pixel
}
