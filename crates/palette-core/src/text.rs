//! Bounded, GUI-independent text layout and grayscale glyph rasterization.
//! Text is previewed transactionally and committed to a new raster layer.
use crate::{
    composite::over,
    document::{BlendMode, Document},
    editing::coverage,
    raster::Raster,
};
use fontdue::{
    Font, FontSettings,
    layout::{CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle},
};
use std::collections::HashMap;

/// Bound preview allocation and layout work before processing user input.
pub const MAX_TEXT_CHARACTERS: usize = 4096;
/// Maximum width/height of one text box, in image pixels.
pub const MAX_TEXT_DIMENSION: u32 = 2048;

/// Horizontal placement of wrapped lines within the text box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Text box dimensions are image pixels, independent of viewport zoom or DPI.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpec {
    pub text: String,
    pub font: usize,
    pub size: f32,
    pub line_height: f32,
    pub alignment: Alignment,
    pub width: u32,
    pub height: u32,
    pub color: [u8; 4],
    pub opacity: f32,
    pub background: Option<[u8; 4]>,
}

/// Reusable parsed fonts. The caller supplies licensed font bytes; no GUI,
/// platform font discovery or native library is involved in core rendering.
pub struct TextEngine {
    fonts: Vec<Font>,
}

impl TextEngine {
    /// Parse a nonempty set of TrueType/OpenType fonts once, for worker reuse.
    pub fn new(fonts: &[&[u8]]) -> Result<Self, String> {
        if fonts.is_empty() {
            return Err("At least one text font is required".into());
        }
        Ok(Self {
            fonts: fonts
                .iter()
                .map(|bytes| {
                    Font::from_bytes(*bytes, FontSettings::default()).map_err(str::to_owned)
                })
                .collect::<Result<_, _>>()?,
        })
    }

    /// Rasterize basic left-to-right Unicode with wrapping and explicit breaks.
    /// Complex shaping/bidirectional layout is a separate future capability.
    /// Bounds and content limits apply before glyph allocation or layout.
    pub fn render(&self, spec: &TextSpec) -> Result<Raster, String> {
        if spec.font >= self.fonts.len()
            || !spec.size.is_finite()
            || !(6.0..=256.0).contains(&spec.size)
            || !spec.line_height.is_finite()
            || !(1.0..=3.0).contains(&spec.line_height)
            || !spec.opacity.is_finite()
            || !(0.0..=1.0).contains(&spec.opacity)
            || spec.width == 0
            || spec.height == 0
            || spec.width > MAX_TEXT_DIMENSION
            || spec.height > MAX_TEXT_DIMENSION
            || spec.text.chars().count() > MAX_TEXT_CHARACTERS
        {
            return Err("Text needs a valid font, 6–256 px size, a box up to 2048 × 2048 and at most 4096 characters".into());
        }
        let mut background = spec.background.unwrap_or([0; 4]);
        background[3] = (f32::from(background[3]) * spec.opacity).round() as u8;
        let mut pixels = Raster::new(spec.width, spec.height, background)?;
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width: Some(spec.width as f32),
            horizontal_align: match spec.alignment {
                Alignment::Left => HorizontalAlign::Left,
                Alignment::Center => HorizontalAlign::Center,
                Alignment::Right => HorizontalAlign::Right,
            },
            line_height: spec.line_height,
            ..Default::default()
        });
        layout.append(
            &self.fonts,
            &TextStyle::new(&spec.text, spec.size, spec.font),
        );
        let mut glyph_cache = HashMap::new();
        for glyph in layout.glyphs() {
            let left = glyph.x.floor() as i64;
            let top = glyph.y.floor() as i64;
            let x0 = left.max(0);
            let y0 = top.max(0);
            let x1 = (left + glyph.width as i64).min(i64::from(spec.width));
            let y1 = (top + glyph.height as i64).min(i64::from(spec.height));
            if x1 <= x0 || y1 <= y0 {
                continue;
            }
            let (_, bitmap) = glyph_cache
                .entry(glyph.key)
                .or_insert_with(|| self.fonts[spec.font].rasterize_config(glyph.key));
            for y in y0..y1 {
                for x in x0..x1 {
                    // Coverage is linear alpha, never a multiplier on straight
                    // RGB. This preserves colored, partly transparent edges.
                    let alpha = bitmap[(y - top) as usize * glyph.width + (x - left) as usize];
                    if alpha != 0 {
                        let x = x as u32;
                        let y = y as u32;
                        pixels.set(
                            x,
                            y,
                            over(
                                pixels.get(x, y),
                                spec.color,
                                f32::from(alpha) / 255.0 * spec.opacity,
                                BlendMode::Normal,
                            ),
                        );
                    }
                }
            }
        }
        Ok(pixels)
    }
}

/// Apply the canvas selection to the final text/background alpha once. Preview
/// and commit share these pixels, including fractional selection coverage.
pub fn constrain(pixels: &mut Raster, document: &Document, origin: [i32; 2]) {
    for y in 0..pixels.height() {
        for x in 0..pixels.width() {
            let mut pixel = pixels.get(x, y);
            if pixel[3] == 0 {
                continue;
            }
            let amount = coverage(
                document.selection.as_ref(),
                i64::from(origin[0]) + i64::from(x),
                i64::from(origin[1]) + i64::from(y),
                document.width,
                document.height,
            );
            pixel[3] = (f32::from(pixel[3]) * amount).round() as u8;
            pixels.set(x, y, pixel);
        }
    }
}

/// Add already constrained preview pixels as an independent topmost layer.
/// The caller wraps a successful insertion in one history command. Empty text
/// and fully masked output are no-ops; other layers are never modified.
pub fn insert(
    document: &mut Document,
    pixels: Raster,
    origin: [i32; 2],
    name: &str,
) -> Result<bool, String> {
    pixels.validate()?;
    if !(0..pixels.height()).any(|y| (0..pixels.width()).any(|x| pixels.get(x, y)[3] != 0)) {
        return Ok(false);
    }
    if document.layers.len() >= 256 {
        return Err("A document can contain at most 256 layers".into());
    }
    document.add_layer();
    let layer = document.active_mut();
    layer.source = pixels;
    layer.offset = origin;
    layer.name = name.chars().take(64).collect();
    Ok(true)
}
