//! Tab at a table's final data cell creates a row, through real keyboard input.
use egui::{Event, Key, Modifiers};
use gridcraft_engine::core::{CellRef, RangeRef, Value};
use serde_json::json;
type Harness = egui_kittest::Harness<'static, gridcraft_ui_egui::SheetApp>;

fn fixture() -> Harness {
    let mut s = crate::support::blank_session();
    s.execute("range.setValues", json!({"range":"B2", "values":[["Item","Qty","Price"],["Tea",1,2],["Coffee",3,4]]})).unwrap();
    s.execute("home.formatAsTable", json!({"range":"B2:D4", "header":true})).unwrap();
    s.execute("selection.set", json!({"cell":"D4"})).unwrap();
    crate::support::harness(s, 4)
}
fn range(h: &Harness) -> RangeRef {
    h.state().session.doc().unwrap().wb.active().unwrap().tables[0].range
}
fn active(h: &Harness) -> CellRef {
    h.state().session.doc().unwrap().selection.active
}
fn tab(h: &mut Harness) {
    h.key_press(Key::Tab);
    h.run_steps(3);
}

#[test]
fn tab_from_last_cell_appends_a_formatted_empty_row_and_undo_redo_restore_it() {
    let mut h = fixture();
    let style = h.state().session.doc().unwrap().wb.active().unwrap().tables[0].style.clone();
    tab(&mut h);
    assert_eq!(range(&h), RangeRef::parse("B2:D5").unwrap());
    assert_eq!(active(&h), CellRef::parse("B5").unwrap());
    let sh = h.state().session.doc().unwrap().wb.active().unwrap();
    assert_eq!(sh.tables[0].style, style);
    for c in ["B5", "C5", "D5"] {
        assert!(sh.value(CellRef::parse(c).unwrap()).is_empty());
    }
    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert_eq!(range(&h), RangeRef::parse("B2:D4").unwrap());
    assert_eq!(active(&h), CellRef::parse("D4").unwrap());
    h.state_mut().run("edit.redo", json!({})).unwrap();
    assert_eq!(range(&h), RangeRef::parse("B2:D5").unwrap());
    // Redo follows the engine's established rule: return to the command's original cells.
    assert_eq!(active(&h), CellRef::parse("D4").unwrap());
}

#[test]
fn editing_tab_commits_then_appends_but_arrow_right_does_not_append() {
    for key in [Key::Tab, Key::ArrowRight] {
        let mut h = fixture();
        h.input_mut().events.push(Event::Text("42".into()));
        h.run_steps(3);
        h.key_press(key);
        h.run_steps(3);
        assert_eq!(h.state().session.doc().unwrap().wb.active().unwrap().value(CellRef::parse("D4").unwrap()), Value::Number(42.0));
        assert_eq!(active(&h), CellRef::parse(if key == Key::Tab { "B5" } else { "E4" }).unwrap());
        assert_eq!(range(&h).end.row, if key == Key::Tab { 4 } else { 3 });
    }
}

#[test]
fn tab_wraps_inside_the_table_and_shift_tab_never_appends() {
    let mut h = fixture();
    h.state_mut().run("selection.set", json!({"cell":"D3"})).unwrap();
    tab(&mut h);
    assert_eq!(active(&h), CellRef::parse("B4").unwrap());
    h.input_mut().events.extend([
        Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::SHIFT },
        Event::Key { key: Key::Tab, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::SHIFT },
    ]);
    h.run_steps(3);
    assert_eq!(active(&h), CellRef::parse("D3").unwrap());
    assert_eq!(range(&h), RangeRef::parse("B2:D4").unwrap());
}

#[test]
fn tab_preserves_cells_below_the_table_and_other_columns() {
    let mut h = fixture();
    h.state_mut().run("range.setValues", json!({"range":"B5", "values":[["below",8,9,"beside"]]})).unwrap();
    tab(&mut h);
    let sh = h.state().session.doc().unwrap().wb.active().unwrap();
    assert_eq!(sh.value(CellRef::parse("B6").unwrap()), Value::text("below"));
    assert_eq!(sh.value(CellRef::parse("E5").unwrap()), Value::text("beside"));
    assert!(sh.value(CellRef::parse("B5").unwrap()).is_empty());
}

#[test]
fn tab_inserts_a_data_row_before_totals_and_leaves_totals_intact() {
    let mut h = fixture();
    h.state_mut().run("table.totalRow", json!({"on":true})).unwrap();
    let before = range(&h);
    let data_end = h.state().session.doc().unwrap().wb.active().unwrap().tables[0].data_range().unwrap().end;
    h.state_mut().run("selection.set", json!({"cell":data_end.a1()})).unwrap();
    tab(&mut h);
    let sh = h.state().session.doc().unwrap().wb.active().unwrap();
    assert!(sh.tables[0].totals_row);
    assert_eq!(sh.tables[0].range.end.row, before.end.row + 1);
    assert_eq!(active(&h), CellRef::new(data_end.row + 1, before.start.col));
    assert!(sh.value(active(&h)).is_empty());
}

#[test]
fn protected_table_refuses_growth_without_moving_the_selection() {
    let mut h = fixture();
    h.state_mut().run("review.protectSheet", json!({})).unwrap();
    tab(&mut h);
    assert_eq!(range(&h), RangeRef::parse("B2:D4").unwrap());
    assert_eq!(active(&h), CellRef::parse("D4").unwrap());
    assert!(h.state().message.is_some());
}

#[test]
fn one_column_table_can_be_extended_repeatedly_and_the_sheet_limit_is_safe() {
    let mut s = crate::support::blank_session();
    s.execute("range.setValues", json!({"range":"G2", "values":[["Header"],[1]]})).unwrap();
    s.execute("home.formatAsTable", json!({"range":"G2:G3", "header":true})).unwrap();
    s.execute("selection.set", json!({"cell":"G3"})).unwrap();
    let mut h = crate::support::harness(s, 4);
    tab(&mut h);
    tab(&mut h);
    assert_eq!(range(&h), RangeRef::parse("G2:G5").unwrap());
    assert_eq!(active(&h), CellRef::parse("G5").unwrap());
    h.state_mut().run("table.resize", json!({"range":"G2:G1048576"})).unwrap();
    assert!(h.state_mut().session.execute("table.appendRow", json!({})).is_err());
    assert_eq!(range(&h).end.row, gridcraft_engine::core::MAX_ROWS - 1);
    assert_eq!(active(&h), CellRef::parse("G5").unwrap());
}

#[test]
fn appended_row_keeps_number_formats_and_calculated_column_references() {
    let mut h = fixture();
    h.state_mut().run("home.formatCells", json!({"range":"D4", "style":{"num_fmt":"0.00"}})).unwrap();
    let wb = std::sync::Arc::make_mut(&mut h.state_mut().session.doc_mut().unwrap().wb);
    wb.sheet_mut(0).unwrap().tables[0].columns[2].formula = Some("C3*2".into());
    tab(&mut h);
    let d = h.state().session.doc().unwrap();
    let sh = d.wb.active().unwrap();
    let appended = sh.cell(CellRef::parse("D5").unwrap()).unwrap();
    assert_eq!(appended.formula.as_ref().unwrap().text, "C5*2");
    assert_eq!(sh.style_id(CellRef::parse("D4").unwrap()), appended.style);
}
