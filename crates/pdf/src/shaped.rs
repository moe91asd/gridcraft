//! Native complex text uses installed OpenType fonts. Glyph outlines make the PDF independent
//! of the reader's fonts; ActualText retains the original logical Unicode string.
use super::{Font, n};
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};
use std::fmt::Write as _;
use std::sync::OnceLock;

fn fonts(bold: bool) -> &'static [Vec<u8>] {
    static REGULAR: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
    static BOLD: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
    let slot = if bold { &BOLD } else { &REGULAR };
    slot.get_or_init(|| {
        #[cfg(target_arch = "wasm32")]
        let paths: Vec<String> = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        let paths = {
            let win = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
            let mut paths = Vec::new();
            if bold {
                paths.extend([format!("{win}/Fonts/tahomabd.ttf"), format!("{win}/Fonts/arialbd.ttf")]);
            }
            paths.extend([
                format!("{win}/Fonts/tahoma.ttf"),
                format!("{win}/Fonts/arial.ttf"),
                "/System/Library/Fonts/GeezaPro.ttc".into(),
                "/System/Library/Fonts/Supplemental/Geeza Pro.ttc".into(),
                "/System/Library/Fonts/Supplemental/Arial.ttf".into(),
                "/System/Library/Fonts/ArialHB.ttc".into(),
            ]);
            for dir in [
                "/usr/share/fonts/truetype/dejavu",
                "/usr/share/fonts/TTF",
                "/usr/share/fonts/truetype/noto",
                "/usr/share/fonts/noto",
                "/run/host/fonts/truetype/dejavu",
                "/run/host/fonts/truetype/noto",
            ] {
                if bold {
                    paths.push(format!("{dir}/DejaVuSans-Bold.ttf"));
                }
                for face in ["DejaVuSans.ttf", "NotoSansArabic-Regular.ttf", "NotoNaskhArabic-Regular.ttf"] {
                    paths.push(format!("{dir}/{face}"));
                }
            }
            paths
        };
        paths
            .iter()
            .filter_map(|p| {
                if std::fs::metadata(p).ok()?.len() > 16 * 1024 * 1024 {
                    return None;
                }
                let bytes = std::fs::read(p).ok()?;
                FontRef::from_index(&bytes, 0).ok()?;
                Some(bytes)
            })
            .take(6)
            .collect()
    })
}

pub(super) struct Shaped {
    pub width: f32,
    pub paths: String,
}

pub(super) fn layout(font: Font, size: f32, text: &str, draw: bool) -> Option<Shaped> {
    if text.len() > 128 * 1024 || !text.chars().any(super::needs_cid) {
        return None;
    }
    let bold = matches!(
        font,
        Font::HelveticaBold | Font::HelveticaBoldOblique | Font::TimesBold | Font::TimesBoldItalic | Font::CourierBold | Font::CourierBoldOblique
    );
    let face = fonts(bold).iter().find_map(|data| {
        let face = FontRef::from_index(data, 0).ok()?;
        let map = face.charmap();
        text.chars().all(|c| c.is_control() || map.map(c).is_some()).then_some(face)
    })?;
    let data = harfrust::ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let scale = size / face.metrics(Size::unscaled(), LocationRef::default()).units_per_em.max(1) as f32;
    let bidi = unicode_bidi::BidiInfo::new(text, None);
    let mut pen = Pen::default();
    let mut advance = 0.0;
    for para in &bidi.paragraphs {
        let (levels, runs) = bidi.visual_runs(para, para.range.clone());
        for run in runs {
            let rtl = levels.get(run.start).is_some_and(|l| l.is_rtl());
            let mut buffer = harfrust::UnicodeBuffer::new();
            buffer.push_str(text.get(run.clone())?);
            buffer.set_direction(if rtl { harfrust::Direction::RightToLeft } else { harfrust::Direction::LeftToRight });
            buffer.guess_segment_properties();
            let shaped = shaper.shape(buffer, harfrust::ShapeOptions::new());
            for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                if draw {
                    pen.offset = (advance + pos.x_offset as f32 * scale, pos.y_offset as f32 * scale);
                    if let Some(glyph) = face.outline_glyphs().get(GlyphId::new(info.glyph_id)) {
                        glyph.draw(DrawSettings::unhinted(Size::new(size), LocationRef::default()), &mut pen).ok()?;
                    }
                    if pen.paths.len() > 8 * 1024 * 1024 {
                        return None;
                    }
                }
                advance += pos.x_advance as f32 * scale;
            }
        }
    }
    Some(Shaped { width: advance, paths: pen.paths })
}

#[derive(Default)]
struct Pen {
    paths: String,
    offset: (f32, f32),
    last: (f32, f32),
}
impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.last = (x, y);
        let _ = writeln!(self.paths, "{} {} m", n(x + self.offset.0), n(y + self.offset.1));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.last = (x, y);
        let _ = writeln!(self.paths, "{} {} l", n(x + self.offset.0), n(y + self.offset.1));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (lx, ly) = self.last;
        self.curve_to(lx + (cx - lx) * 2.0 / 3.0, ly + (cy - ly) * 2.0 / 3.0, x + (cx - x) * 2.0 / 3.0, y + (cy - y) * 2.0 / 3.0, x, y);
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        let (ox, oy) = self.offset;
        let _ = writeln!(self.paths, "{} {} {} {} {} {} c", n(a + ox), n(b + oy), n(c + ox), n(d + oy), n(x + ox), n(y + oy));
        self.last = (x, y);
    }
    fn close(&mut self) {
        self.paths.push_str("h\n");
    }
}
