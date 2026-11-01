//! Synthetic RTL print fixture, using the same export command as File > Export > PDF.
use gridcraft_engine::Session;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("provide an output PDF path")?;
    let mut s = Session::new();
    s.new_workbook();
    s.execute("view.rightToLeft", json!({"on":true}))?;
    s.execute(
        "range.setValues",
        json!({"range":"A1", "values":[
            ["المنتج", "الكمية", "الهاتف", "الإجمالي"],
            ["قهوة عربية", "3.0 kg", "'0708642660", 51.0],
            ["شاي أخضر", "1.5 kg", "'0708446495", 25.5],
            ["שלום 123", "1.0 kg", "'0778333333", 17.0],
            ["مرحبا بالعالم - 123", "", "", 93.5]
        ]}),
    )?;
    s.execute("home.columnWidth", json!({"cols":"A:D", "width":150}))?;
    s.execute("home.rowHeight", json!({"rows":"1:5", "height":36}))?;
    s.execute("home.bold", json!({"range":"A1:D1"}))?;
    s.execute("pageLayout.pageSetup", json!({"printArea":"A1:D5", "gridlines":true, "headings":true, "fitToPage":true}))?;
    s.execute("file.exportPdf", json!({"path":path}))?;
    Ok(())
}
