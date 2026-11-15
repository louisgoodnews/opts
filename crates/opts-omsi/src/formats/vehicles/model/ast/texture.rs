//! Script-driven textures of a vehicle model: text, script and colour-change textures.

/// Dynamic text texture rendered from a script variable (`[texttexture]`, `[texttexture_enh]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextTexture {
    /// Script string variable name supplying text.
    pub variable: String,
    /// Bitmap font file or font ident.
    pub font_name: String,
    /// Raster texture width in pixels.
    pub width: u32,
    /// Raster texture height in pixels.
    pub height: u32,
    /// Monochrome (`false`) or full color (`true`).
    pub full_color: bool,
    /// User color in RGB (0-255).
    pub color_rgb: [u8; 3],
    /// Enhanced alignment: 0 = centered, 1 = left, 2 = right.
    pub orientation: Option<u32>,
    /// Pixel grid alignment.
    pub grid: Option<u32>,
}

/// Script-generated texture (`[scripttexture]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptTexture {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
}

/// Color texture change scheme (`[CTC]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtcScheme {
    /// Name of the color scheme.
    pub name: String,
    /// Texture directory path.
    pub texture_path: String,
    /// Default index.
    pub default_index: i32,
}

/// Color texture change texture declaration (`[CTCTexture]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtcTexture {
    /// Texture ident.
    pub name: String,
    /// Texture file path.
    pub texture_path: String,
}
