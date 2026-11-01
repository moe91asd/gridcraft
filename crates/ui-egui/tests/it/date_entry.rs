//! Enter stores a date serial using the destination format, through both editors.
use egui::{Event, Key, Shape};
use gridcraft_engine::core::{CellRef, date::serial_from_ymd, parse::current_year};
use serde_json::json;

#[test]
fn date_entry_enter_in_grid_and_formula_bar_preserves_format_and_renders_date() {
    for from_bar in [false, true] {
        for (code, month, day) in [("dd/mm/yyyy", 11, 1), ("mm/dd/yyyy", 1, 11)] {
            let mut s = crate::support::blank_session();
            s.execute("home.formatCells", json!({"range":"A:A","style":{"num_fmt":code}})).unwrap();
            s.execute("home.columnWidth", json!({"cols":"A:A","width":180})).unwrap();
            let mut h = crate::support::harness(s, 4);
            h.state_mut().begin_edit(Some(String::new()), from_bar);
            h.run_steps(2);
            h.input_mut().events.push(Event::Text("1-11".into()));
            h.run_steps(3);
            h.key_press(Key::Enter);
            h.run_steps(3);
            assert!(h.state().editor.is_none());
            let d = h.state().session.doc().unwrap();
            let sh = d.wb.active().unwrap();
            let at = CellRef::parse("A1").unwrap();
            assert_eq!(sh.value(at).as_f64(), serial_from_ymd(d.wb.date_system, current_year(), month, day));
            assert_eq!(d.wb.styles.get(sh.style_id(at)).num_fmt.as_str(), code);
            let expected = format!("01/11/{}", current_year());
            assert_eq!(gridcraft_engine::display::value_text(&d.wb, sh, at), expected);
            assert!(h.output().shapes.iter().any(|s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text == expected)));
        }
    }
}

#[test]
fn date_entry_in_an_appended_table_row_uses_its_inherited_date_column_format() {
    let mut s = crate::support::blank_session();
    s.execute("range.setValues", json!({"range":"A1","values":[["Date","Amount"],["2026-10-01",5]]})).unwrap();
    s.execute("home.formatAsTable", json!({"range":"A1:B2","header":true})).unwrap();
    s.execute("home.formatCells", json!({"range":"A2","style":{"num_fmt":"dd/mm/yyyy"}})).unwrap();
    s.execute("selection.set", json!({"cell":"B2"})).unwrap();
    let mut h = crate::support::harness(s, 4);
    h.key_press(Key::Tab);
    h.run_steps(3);
    assert_eq!(h.state().session.doc().unwrap().selection.active, CellRef::parse("A3").unwrap());
    h.input_mut().events.push(Event::Text("1-11".into()));
    h.run_steps(3);
    h.key_press(Key::Enter);
    h.run_steps(3);
    let d = h.state().session.doc().unwrap();
    let sh = d.wb.active().unwrap();
    let at = CellRef::parse("A3").unwrap();
    assert_eq!(sh.value(at).as_f64(), serial_from_ymd(d.wb.date_system, current_year(), 11, 1));
    assert_eq!(d.wb.styles.get(sh.style_id(at)).num_fmt.as_str(), "dd/mm/yyyy");
}
