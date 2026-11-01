//! Paragraph direction is independent of worksheet geometry. Never reverse stored text:
//! epaint shapes and reorders the original Unicode string, retaining logical cursor indices.

use egui::Align;
use gridcraft_engine::model::HAlign;

pub(crate) fn alignment(text: &str) -> Align {
    if unicode_bidi::get_base_direction(text) == unicode_bidi::Direction::Rtl { Align::RIGHT } else { Align::LEFT }
}

pub(crate) fn cell_alignment(h: HAlign, text: &str) -> Align {
    if text.starts_with('=') {
        return Align::LEFT;
    }
    match h {
        HAlign::Right => Align::RIGHT,
        HAlign::Center | HAlign::CenterAcross | HAlign::Distributed => Align::Center,
        HAlign::General => alignment(text),
        _ => Align::LEFT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_strong_character_sets_alignment_without_reversing_numbers_or_isolates() {
        for text in ["مرحبا 123", "  (123) שלום", "\u{2066}Latin\u{2069} عربي"] {
            assert_eq!(alignment(text), Align::RIGHT, "{text}");
        }
        for text in ["Latin عربي", "=SUM('ورقة'!A1)", "123", ""] {
            assert_eq!(alignment(text), Align::LEFT, "{text}");
        }
    }
}
