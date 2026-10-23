//! Bounded, GUI-independent text layout and grayscale glyph rasterization.
//! Text is previewed transactionally and committed to a new raster layer.
use crate::{
    composite::over,
    document::{BlendMode, Document},
    editing::coverage,
    raster::Raster,
};
mod style;
use fontdue::{
    Font, FontSettings,
    layout::{
        CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle as LayoutTextStyle,
    },
};
use std::{collections::HashMap, ops::Range, sync::Arc};
pub use style::{TextGlyph, TextStyle};

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
    pub style: TextStyle,
    /// Per-Unicode-character formatting; empty means uniform `style`.
    pub character_styles: Vec<TextStyle>,
    pub font: usize,
    pub size: f32,
    pub line_height: f32,
    pub alignment: Alignment,
    pub width: u32,
    pub height: u32,
    /// Visible vertical viewport offset in image pixels; draft text remains intact.
    pub scroll_y: f32,
    pub color: [u8; 4],
    pub opacity: f32,
    pub background: Option<[u8; 4]>,
}

/// A contiguous UTF-8 run. Platform callers may resolve its requested style to
/// a real face before passing the runs to `TextEngine::render_runs`.
#[derive(Clone, Debug)]
pub struct TextRun {
    pub range: Range<usize>,
    pub font: usize,
    pub style: TextStyle,
}
impl TextSpec {
    /// Group character formatting into byte ranges without splitting Unicode.
    pub fn runs(&self) -> Vec<TextRun> {
        let mut runs: Vec<TextRun> = Vec::new();
        for (index, (byte, ch)) in self.text.char_indices().enumerate() {
            let style = self
                .character_styles
                .get(index)
                .copied()
                .unwrap_or(self.style);
            if let Some(last) = runs.last_mut().filter(|last| last.style == style) {
                last.range.end = byte + ch.len_utf8();
            } else {
                runs.push(TextRun {
                    range: byte..byte + ch.len_utf8(),
                    font: self.font,
                    style,
                });
            }
        }
        runs
    }
}

/// Reusable parsed fonts. The caller supplies licensed font bytes; no GUI,
/// platform font discovery or native library is involved in core rendering.
pub struct TextEngine {
    fonts: Vec<Arc<Font>>,
}

impl TextEngine {
    /// Rasterize one glyph with the same bounded synthetic styling as the CPU
    /// text renderer. The desktop font atlas uses this for missing style faces.
    pub fn glyph(
        &self,
        font: usize,
        character: char,
        size: f32,
        style: TextStyle,
    ) -> Result<TextGlyph, String> {
        let font = self.fonts.get(font).ok_or("Invalid text font")?;
        style::glyph(font, font.lookup_glyph_index(character), size, style).map(|(_, glyph)| glyph)
    }
    /// Parse a nonempty set of TrueType/OpenType fonts once, for worker reuse.
    pub fn new(fonts: &[&[u8]]) -> Result<Self, String> {
        if fonts.is_empty() {
            return Err("At least one text font is required".into());
        }
        Ok(Self {
            fonts: fonts
                .iter()
                .map(|bytes| {
                    Font::from_bytes(*bytes, FontSettings::default())
                        .map(Arc::new)
                        .map_err(str::to_owned)
                })
                .collect::<Result<_, _>>()?,
        })
    }

    /// Parse one selected collection face. System discovery stays outside core;
    /// the same face index/bytes are used by the GUI preview and raster worker.
    pub fn from_face(bytes: &[u8], collection_index: u32) -> Result<Self, String> {
        Ok(Self {
            fonts: vec![
                Font::from_bytes(
                    bytes,
                    FontSettings {
                        collection_index,
                        ..Default::default()
                    },
                )
                .map(Arc::new)
                .map_err(str::to_owned)?,
            ],
        })
    }

    /// Share parsed faces without reparsing or copying glyph data. Each supplied
    /// engine contributes its first face, in the supplied order.
    pub fn combine(engines: &[Arc<Self>]) -> Self {
        Self {
            fonts: engines
                .iter()
                .map(|engine| engine.fonts[0].clone())
                .collect(),
        }
    }

    /// Rasterize basic left-to-right Unicode with wrapping and explicit breaks.
    /// Complex shaping/bidirectional layout is a separate future capability.
    /// Bounds and content limits apply before glyph allocation or layout.
    pub fn render(&self, spec: &TextSpec) -> Result<Raster, String> {
        self.render_runs(spec, &spec.runs())
    }

    /// Lay out mixed styles together, preserving wrapping, alignment and alpha.
    /// Runs must cover the entire string consecutively at UTF-8 boundaries.
    pub fn render_runs(&self, spec: &TextSpec, runs: &[TextRun]) -> Result<Raster, String> {
        let mut end = 0;
        for run in runs {
            if run.range.start != end
                || run.range.end <= end
                || spec.text.get(run.range.clone()).is_none()
                || run.font >= self.fonts.len()
            {
                return Err("Invalid text formatting run".into());
            }
            end = run.range.end;
        }
        if end != spec.text.len() {
            return Err("Incomplete text formatting runs".into());
        }
        if spec.font >= self.fonts.len()
            || !spec.size.is_finite()
            || !(6.0..=256.0).contains(&spec.size)
            || !spec.line_height.is_finite()
            || !(1.0..=3.0).contains(&spec.line_height)
            || !spec.opacity.is_finite()
            || !(0.0..=1.0).contains(&spec.opacity)
            || !spec.scroll_y.is_finite()
            || !(0.0..=MAX_TEXT_CHARACTERS as f32 * 256.0 * 3.0).contains(&spec.scroll_y)
            || spec.width == 0
            || spec.height == 0
            || spec.width > MAX_TEXT_DIMENSION
            || spec.height > MAX_TEXT_DIMENSION
            || spec.text.chars().count() > MAX_TEXT_CHARACTERS
            || (!spec.character_styles.is_empty()
                && spec.character_styles.len() != spec.text.chars().count())
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
        for run in runs {
            layout.append(
                &self.fonts,
                &LayoutTextStyle::with_user_data(
                    &spec.text[run.range.clone()],
                    spec.size,
                    run.font,
                    run.style,
                ),
            );
        }
        // Build one ink-coverage plane before applying color/opacity. A rule
        // crossing a descender or synthetic bold copies must not darken alpha.
        let mut ink = vec![0u8; spec.width as usize * spec.height as usize];
        let mut glyph_cache = HashMap::new();
        for glyph in layout.glyphs() {
            if glyph.char_data.is_control() {
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(entry) =
                glyph_cache.entry((glyph.key, glyph.font_index, glyph.user_data))
            {
                entry.insert(style::glyph(
                    &self.fonts[glyph.font_index],
                    glyph.key.glyph_index,
                    spec.size,
                    glyph.user_data,
                )?);
            }
            let (metrics, bitmap) = &glyph_cache[&(glyph.key, glyph.font_index, glyph.user_data)];
            let left = glyph.x.floor() as i64 + i64::from(bitmap.left - metrics.xmin);
            let top = (glyph.y - spec.scroll_y).floor() as i64
                + i64::from(bitmap.top + metrics.ymin + metrics.height as i32);
            let x0 = left.max(0);
            let y0 = top.max(0);
            let x1 = (left + i64::from(bitmap.width)).min(i64::from(spec.width));
            let y1 = (top + i64::from(bitmap.height)).min(i64::from(spec.height));
            if x1 <= x0 || y1 <= y0 {
                continue;
            }
            for y in y0..y1 {
                for x in x0..x1 {
                    let alpha = bitmap.coverage
                        [(y - top) as usize * bitmap.width as usize + (x - left) as usize];
                    let target = &mut ink[y as usize * spec.width as usize + x as usize];
                    *target = (*target).max(alpha);
                }
            }
        }
        // Join adjacent decorated advances, but never extend a rule into an
        // unformatted run. Glyph coverage and rules share one alpha plane.
        for line in layout.lines().into_iter().flatten() {
            for strike in [false, true] {
                let mut extent: Option<(f32, f32)> = None;
                let y = if strike {
                    line.baseline_y - (line.max_ascent + line.min_descent) * 0.5
                } else {
                    line.baseline_y - line.min_descent
                } - spec.scroll_y;
                for glyph in &layout.glyphs()[line.glyph_start..=line.glyph_end] {
                    let enabled = if strike {
                        glyph.user_data.strikethrough
                    } else {
                        glyph.user_data.underline
                    };
                    if enabled && !glyph.char_data.is_control() {
                        let metrics = self.fonts[glyph.font_index]
                            .metrics_indexed(glyph.key.glyph_index, spec.size);
                        let pen = glyph.x - metrics.xmin as f32;
                        let end = pen + metrics.advance_width.ceil();
                        extent = Some(extent.map_or((pen, end), |(a, b)| (a.min(pen), b.max(end))));
                    } else if let Some((left, right)) = extent.take() {
                        style::rule(&mut ink, spec, left, right, y, (spec.size / 16.).max(1.));
                    }
                }
                if let Some((left, right)) = extent {
                    style::rule(&mut ink, spec, left, right, y, (spec.size / 16.).max(1.));
                }
            }
        }
        for (index, alpha) in ink.into_iter().enumerate().filter(|(_, a)| *a != 0) {
            let x = index as u32 % spec.width;
            let y = index as u32 / spec.width;
            pixels.set(
                x,
                y,
                over(
                    pixels.get(x, y),
                    spec.color,
                    f32::from(alpha) / 255. * spec.opacity,
                    BlendMode::Normal,
                ),
            );
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
