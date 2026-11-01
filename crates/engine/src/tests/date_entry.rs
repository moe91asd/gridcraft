use super::*;
use gridcraft_core::{DateSystem, date::serial_from_ymd, parse::current_year};

#[test]
fn date_entry_honors_effective_cell_row_and_column_formats_and_date_system() {
    for sys in [DateSystem::D1900, DateSystem::D1904] {
        for (range, cell, code, month, day) in
            [("A1", "A1", "dd/mm/yyyy", 11, 1), ("2:2", "B2", "mm/dd/yyyy", 1, 11), ("C:C", "C7", "dd mmm yyyy", 11, 1)]
        {
            let mut s = s();
            std::sync::Arc::make_mut(&mut s.doc_mut().unwrap().wb).date_system = sys;
            s.execute("home.formatCells", json!({"range":range, "style":{"num_fmt":code}})).unwrap();
            s.execute("cell.set", json!({"cell":cell,"input":"1-11"})).unwrap();
            assert_eq!(v(&s, cell).as_f64(), serial_from_ymd(sys, current_year(), month, day));
            let d = s.doc().unwrap();
            let sh = d.wb.active().unwrap();
            assert_eq!(d.wb.styles.get(sh.style_id(CellRef::parse(cell).unwrap())).num_fmt.as_str(), code);
            s.execute("edit.undo", json!({})).unwrap();
            assert!(v(&s, cell).is_empty());
            s.execute("edit.redo", json!({})).unwrap();
            assert_eq!(v(&s, cell).as_f64(), serial_from_ymd(sys, current_year(), month, day));
        }
    }
}

#[test]
fn date_entry_keeps_general_text_number_and_formula_behavior() {
    let mut s = s();
    s.execute("home.formatCells", json!({"range":"A1:A5", "style":{"num_fmt":"dd/mm/yyyy"}})).unwrap();
    for (cell, input) in [("A1", "'1-11"), ("A2", "11-13"), ("A3", "42"), ("A4", "=1-11"), ("A5", "2026-11-01"), ("B1", "1-11")] {
        s.execute("cell.set", json!({"cell":cell,"input":input})).unwrap();
    }
    assert_eq!(v(&s, "A1"), Value::text("1-11"));
    assert_eq!(v(&s, "A2"), Value::text("11-13"));
    assert_eq!(v(&s, "A3"), Value::Number(42.0));
    assert_eq!(v(&s, "A4"), Value::Number(-10.0));
    assert_eq!(v(&s, "A5").as_f64(), serial_from_ymd(DateSystem::D1900, 2026, 11, 1));
    assert_eq!(v(&s, "B1").as_f64(), serial_from_ymd(DateSystem::D1900, current_year(), 1, 11));
    s.execute("home.formatCells", json!({"range":"C1","style":{"num_fmt":"@"}})).unwrap();
    s.execute("cell.set", json!({"cell":"C1","input":"1-11"})).unwrap();
    assert_eq!(v(&s, "C1"), Value::text("1-11"));
}

#[test]
fn date_validation_checks_the_same_interpretation_as_cell_entry() {
    let mut s = s();
    let year = current_year();
    s.execute("home.formatCells", json!({"range":"A1","style":{"num_fmt":"dd/mm/yyyy"}})).unwrap();
    s.execute("home.formatCells", json!({"range":"B1","style":{"num_fmt":"mm/dd/yyyy"}})).unwrap();
    s.execute(
        "data.validation",
        json!({"range":"A1:B1","type":"date","operator":"between","formula1":format!("DATE({year},10,1)"),"formula2":format!("DATE({year},12,1)")}),
    )
    .unwrap();
    let wb = &s.doc().unwrap().wb;
    assert!(crate::cmd::data::check_validation(wb, 0, CellRef::parse("A1").unwrap(), "1-11").is_none());
    assert!(crate::cmd::data::check_validation(wb, 0, CellRef::parse("B1").unwrap(), "1-11").is_some());
}
