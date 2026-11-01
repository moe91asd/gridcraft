//! Paragraph direction and worksheet direction remain independent through painting and editing.
use egui::{Event, Key, Shape};
use gridcraft_engine::Session;
use gridcraft_engine::core::CellRef;
use gridcraft_ui_egui::{SheetApp, grid::Geo, theme};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

// RLM is a strong RTL character requiring no font glyph. This exercises the complete bidi
// path even in a minimal build without the optional craft-fonts script faces.
const RTL: &str = "\u{200f}Total 123";

fn harness(text: &str, rtl: bool) -> Harness {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("cell.set", json!({"cell":"C3", "input":text})).unwrap();
    s.execute("view.rightToLeft", json!({"on":rtl})).unwrap();
    s.execute("selection.set", json!({"cell":"C3"})).unwrap();
    crate::support::harness(s, 4)
}

fn text_shape<'a>(h: &'a Harness, text: &str) -> &'a egui::epaint::TextShape {
    let top = h.state().grid.cells_rect.unwrap().top();
    h.output()
        .shapes
        .iter()
        .rev()
        .find_map(|s| match &s.shape {
            Shape::Text(t) if t.galley.job.text == text && t.pos.y >= top => Some(t),
            _ => None,
        })
        .expect("text is painted in the grid")
}

fn cell(h: &Harness) -> egui::Rect {
    let app = h.state();
    let sh = app.session.doc().unwrap().wb.active().unwrap();
    Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll).screen_cell_rect(sh, CellRef::parse("C3").unwrap())
}

#[test]
fn rtl_paragraph_aligns_right_in_either_sheet_direction_and_edits_in_logical_order() {
    for rtl in [false, true] {
        let mut h = harness(RTL, rtl);
        let t = text_shape(&h, RTL);
        assert!(t.galley.rows[0].rtl_paragraph);
        assert!((t.visual_bounding_rect().right() - cell(&h).right()).abs() < 6.0);
        h.key_press(Key::F2);
        h.run_steps(3);
        let t = text_shape(&h, RTL);
        assert!(t.galley.rows[0].rtl_paragraph);
        assert_eq!(t.galley.job.halign, egui::Align::RIGHT);
        assert_eq!(h.state().editor.as_ref().unwrap().text, RTL);
        // A paste at the logical end is preserved verbatim when committed.
        h.input_mut().events.push(Event::Paste("!".into()));
        h.run_steps(2);
        h.key_press(Key::Enter);
        h.run_steps(3);
        assert_eq!(
            h.state().session.doc().unwrap().wb.active().unwrap().value(CellRef::parse("C3").unwrap()).as_text(),
            Some(format!("{RTL}!").as_str())
        );
    }
}

#[test]
fn explicit_left_alignment_keeps_wrapped_rtl_rows_on_the_left() {
    for rtl in [false, true] {
        for (h_align, align) in [("Left", egui::Align::LEFT), ("Right", egui::Align::RIGHT)] {
            let mut h = harness(RTL, rtl);
            h.state_mut().run("home.formatCells", json!({"range":"C3", "style":{"align":{"h":h_align, "wrap":true}}})).unwrap();
            h.run_steps(3);
            let t = text_shape(&h, RTL);
            assert!(!t.galley.job.halign_min_is_start);
            assert_eq!(t.galley.job.halign, align);
            let bounds = t.visual_bounding_rect();
            let diff = if align == egui::Align::LEFT { bounds.left() - cell(&h).left() } else { bounds.right() - cell(&h).right() };
            assert!(diff.abs() < 6.0, "{rtl} {h_align}: {bounds:?}");
        }
    }
}

#[test]
fn arabic_and_hebrew_use_bidi_layout_without_changing_the_cell_value() {
    for text in ["مرحبا بالعالم 123", "שלום עולם 123"] {
        if !theme::craft_fonts_cover(text) {
            eprintln!("RTL script case needs its optional craft-fonts face: {text}");
            continue;
        }
        for rtl in [false, true] {
            let h = harness(text, rtl);
            let t = text_shape(&h, text);
            assert!(t.galley.rows[0].rtl_paragraph);
            let glyphs = &t.galley.rows[0].glyphs;
            let digits: Vec<_> = glyphs.iter().filter(|g| g.chr.is_ascii_digit()).collect();
            assert!(digits.windows(2).all(|w| w[0].pos.x < w[1].pos.x), "digits keep their order");
            assert_eq!(t.galley.job.text, text);
        }
    }
}

#[test]
fn saved_rtl_shape_paragraph_direction_is_used_when_painted_and_edited() {
    let mut s = Session::new();
    s.new_workbook();
    let text = "Latin text (123)";
    let id = s.execute("insert.textBox", json!({"at":"C3", "width":240, "height":100, "text":text})).unwrap()["shape"].as_u64().unwrap() as u32;
    // Synthetic imported shape: no external workbook fixture or font is needed to verify rtl=1.
    let wb = std::sync::Arc::make_mut(&mut s.doc_mut().unwrap().wb);
    wb.sheet_mut(0).unwrap().shapes[0].format.rtl = true;
    let mut h = crate::support::harness(s, 4);
    for editing in [false, true] {
        if editing {
            assert!(h.state_mut().begin_text_box_edit(id));
            h.run_steps(3);
        }
        let t = text_shape(&h, text);
        assert!(t.galley.rows[0].rtl_paragraph);
        assert_eq!(t.galley.job.halign, egui::Align::RIGHT);
    }
}

#[test]
fn formula_with_rtl_string_keeps_a_left_to_right_base_in_both_editors() {
    let formula = format!("=\"{RTL}\"");
    let mut h = harness(&formula, true);
    h.key_press(Key::F2);
    h.run_steps(3);
    for from_bar in [false, true] {
        h.state_mut().editor.as_mut().unwrap().from_formula_bar = from_bar;
        h.run_steps(3);
        let shapes: Vec<_> = h
            .output()
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                Shape::Text(t) if t.galley.job.text == formula => Some(t),
                _ => None,
            })
            .collect();
        assert!(!shapes.is_empty());
        for t in shapes {
            assert!(!t.galley.rows[0].rtl_paragraph);
            assert_eq!(t.galley.job.halign, egui::Align::LEFT);
        }
    }
}
