//! Excel report (stock + movement log) and the import template.

use crate::model::{kind_label, Inventory, Part};
use crate::text::split_ts;
use crate::Result;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet};
use std::path::Path;

pub const STOCK_HEADERS: [&str; 11] = [
    "STT", "Part number", "Name", "Footprint", "Pitch", "Quantity", "Descriptions",
    "Manufacturer", "Manufacturer Part Number", "Product Category", "Vị trí",
];
const STOCK_WIDTHS: [f64; 11] = [6.0, 18.0, 28.0, 14.0, 10.0, 10.0, 40.0, 18.0, 26.0, 20.0, 14.0];

pub const HISTORY_HEADERS: [&str; 12] = [
    "STT", "Ngày", "Thứ", "Giờ", "Thao tác", "Part number", "Manufacturer Part Number", "Name", "Số lượng",
    "Tồn sau", "Người", "Ghi chú",
];
const HISTORY_WIDTHS: [f64; 12] = [6.0, 12.0, 10.0, 10.0, 12.0, 18.0, 26.0, 28.0, 10.0, 10.0, 18.0, 40.0];

/// Case-insensitive natural-ish compare where empty strings sort last.
fn cmp_text(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a.trim().is_empty(), b.trim().is_empty()) {
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

/// Report order: Manufacturer Part Number, then Part number, then Name.
pub fn sorted_parts(parts: &[Part]) -> Vec<&Part> {
    let mut v: Vec<&Part> = parts.iter().collect();
    v.sort_by(|a, b| {
        cmp_text(&a.mpn, &b.mpn)
            .then_with(|| cmp_text(&a.part_number, &b.part_number))
            .then_with(|| cmp_text(&a.name, &b.name))
    });
    v
}

fn header_format() -> Format {
    Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xDDEBF7))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
}

fn write_header(ws: &mut Worksheet, headers: &[&str], widths: &[f64]) -> Result<()> {
    let hf = header_format();
    for (i, h) in headers.iter().enumerate() {
        ws.write_string_with_format(0, i as u16, *h, &hf)?;
        ws.set_column_width(i as u16, widths[i])?;
    }
    ws.set_freeze_panes(1, 0)?;
    Ok(())
}

/// Write the two-sheet report to `path` (overwrites).
pub fn write_report(path: &Path, inv: &Inventory) -> Result<()> {
    let mut wb = Workbook::new();
    let cell = Format::new().set_border(FormatBorder::Thin);
    let num = Format::new().set_border(FormatBorder::Thin).set_num_format("#,##0");
    let qty_ok = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0")
        .set_bold()
        .set_background_color(Color::RGB(0xFFF4CC));
    let qty_out = Format::new()
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0")
        .set_bold()
        .set_font_color(Color::RGB(0x9B1C1C))
        .set_background_color(Color::RGB(0xF8D7DA));

    // --- Sheet 1: stock ---
    let ws = wb.add_worksheet();
    ws.set_name("Ton kho")?;
    write_header(ws, &STOCK_HEADERS, &STOCK_WIDTHS)?;
    for (i, p) in sorted_parts(&inv.parts).into_iter().enumerate() {
        let r = (i + 1) as u32;
        ws.write_number_with_format(r, 0, (i + 1) as f64, &num)?;
        ws.write_string_with_format(r, 1, &p.part_number, &cell)?;
        ws.write_string_with_format(r, 2, &p.name, &cell)?;
        ws.write_string_with_format(r, 3, &p.footprint, &cell)?;
        ws.write_string_with_format(r, 4, &p.pitch, &cell)?;
        let qf = if p.quantity < 1 { &qty_out } else { &qty_ok };
        ws.write_number_with_format(r, 5, p.quantity as f64, qf)?;
        ws.write_string_with_format(r, 6, &p.description, &cell)?;
        ws.write_string_with_format(r, 7, &p.manufacturer, &cell)?;
        ws.write_string_with_format(r, 8, &p.mpn, &cell)?;
        ws.write_string_with_format(r, 9, &p.category, &cell)?;
        ws.write_string_with_format(r, 10, &p.location, &cell)?;
    }
    if !inv.parts.is_empty() {
        ws.autofilter(0, 0, inv.parts.len() as u32, (STOCK_HEADERS.len() - 1) as u16)?;
    }

    // --- Sheet 2: history ---
    let ws = wb.add_worksheet();
    ws.set_name("Lich su")?;
    write_header(ws, &HISTORY_HEADERS, &HISTORY_WIDTHS)?;
    let signed = Format::new().set_border(FormatBorder::Thin).set_num_format("+#,##0;-#,##0;0");
    for (i, h) in inv.history.iter().enumerate() {
        let r = (i + 1) as u32;
        let (date, weekday, time) = split_ts(&h.ts);
        ws.write_number_with_format(r, 0, (i + 1) as f64, &num)?;
        ws.write_string_with_format(r, 1, &date, &cell)?;
        ws.write_string_with_format(r, 2, &weekday, &cell)?;
        ws.write_string_with_format(r, 3, &time, &cell)?;
        ws.write_string_with_format(r, 4, kind_label(&h.kind), &cell)?;
        ws.write_string_with_format(r, 5, &h.part_number, &cell)?;
        ws.write_string_with_format(r, 6, &h.mpn, &cell)?;
        ws.write_string_with_format(r, 7, &h.name, &cell)?;
        ws.write_number_with_format(r, 8, h.delta as f64, &signed)?;
        ws.write_number_with_format(r, 9, h.after as f64, &num)?;
        ws.write_string_with_format(r, 10, &h.person, &cell)?;
        ws.write_string_with_format(r, 11, &h.note, &cell)?;
    }
    if !inv.history.is_empty() {
        ws.autofilter(0, 0, inv.history.len() as u32, (HISTORY_HEADERS.len() - 1) as u16)?;
    }

    wb.save(path)?;
    Ok(())
}

/// Write an empty import template with two example rows.
pub fn write_template(path: &Path) -> Result<()> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Linh kien")?;
    let headers = &STOCK_HEADERS[1..];
    let widths = &STOCK_WIDTHS[1..];
    write_header(ws, headers, widths)?;
    let examples: [[&str; 10]; 2] = [
        ["R-0603-10K", "Điện trở 10kΩ 1%", "0603", "", "500", "Resistor 10k 1% 0603", "Yageo", "RC0603FR-0710KL", "Resistor", "Kệ A - Hộp 1"],
        ["U-STM32F103", "MCU STM32F103C8T6", "LQFP-48", "0.5mm", "12", "ARM Cortex-M3 72MHz", "STMicroelectronics", "STM32F103C8T6", "MCU", "Tủ B - Ngăn 3"],
    ];
    let cell = Format::new().set_border(FormatBorder::Thin);
    for (r, row) in examples.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            if c == 4 {
                ws.write_number_with_format((r + 1) as u32, c as u16, v.parse::<f64>().unwrap_or(0.0), &cell)?;
            } else {
                ws.write_string_with_format((r + 1) as u32, c as u16, *v, &cell)?;
            }
        }
    }
    wb.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::parse_file;
    use crate::model::{HistoryEntry, Part};

    #[test]
    fn report_roundtrips_through_import() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bao_cao.xlsx");
        let mut inv = Inventory::default();
        inv.parts.push(Part {
            id: "1".into(), part_number: "R-10K".into(), name: "Điện trở 10k".into(), footprint: "0603".into(),
            pitch: "".into(), quantity: 1500, description: "1%".into(), manufacturer: "Yageo".into(),
            mpn: "RC0603FR-0710KL".into(), category: "Resistor".into(), location: "Kệ A1".into(),
        });
        inv.parts.push(Part { id: "2".into(), part_number: "U-1".into(), name: "MCU".into(), pitch: "0.5mm".into(), quantity: 0, ..Default::default() });
        inv.history.push(HistoryEntry {
            id: "h1".into(), ts: "2026-11-11T09:15:00".into(), kind: "take".into(), part_id: "1".into(),
            part_number: "R-10K".into(), mpn: "RC0603FR-0710KL".into(), name: "Điện trở 10k".into(), delta: -20, after: 1500, person: "Sang".into(), note: "".into(),
        });
        write_report(&p, &inv).unwrap();

        // The stock sheet must import back with the same values.
        let r = parse_file(&p).unwrap();
        assert_eq!(r.sheet, "Ton kho");
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.rows[0].part_number, "R-10K");
        assert_eq!(r.rows[0].quantity, Some(1500));
        assert_eq!(r.rows[0].mpn, "RC0603FR-0710KL");
        assert_eq!(r.rows[0].manufacturer, "Yageo");
        assert_eq!(r.rows[0].location, "Kệ A1");
        assert_eq!(r.rows[1].pitch, "0.5mm");
        assert_eq!(r.rows[1].quantity, Some(0));
        assert_eq!(r.mapping.len(), 10, "{:?}", r.mapping);

        // History sheet has Ngày / Thứ / Giờ split out.
        use calamine::{open_workbook, Reader, Xlsx};
        let mut wb: Xlsx<_> = open_workbook(&p).unwrap();
        let range = wb.worksheet_range("Lich su").unwrap();
        let row: Vec<String> = range.rows().nth(1).unwrap().iter().map(|c| c.to_string()).collect();
        assert_eq!(&row[1..5], &["11/11/2026", "Thứ tư", "09:15:00", "Lấy ra"]);
        assert_eq!(row[6], "RC0603FR-0710KL");
        assert_eq!(row[8], "-20");
        assert_eq!(row[10], "Sang");
    }

    #[test]
    fn report_sorted_by_mpn_and_same_part_number_kept_apart() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sort.xlsx");
        let mk = |id: &str, pn: &str, mpn: &str, q: i64| Part {
            id: id.into(), part_number: pn.into(), mpn: mpn.into(), name: format!("n{id}"), quantity: q, ..Default::default()
        };
        let mut inv = Inventory::default();
        inv.parts = vec![
            mk("1", "C-100N", "GRM188R71H104KA93D", 50),
            mk("2", "C-100N", "CL10B104KB8NNNC", 0),
            mk("3", "A-1", "", 3),
            mk("4", "B-1", "ABC-1", 7),
        ];
        write_report(&p, &inv).unwrap();
        let r = parse_file(&p).unwrap();
        let order: Vec<&str> = r.rows.iter().map(|x| x.mpn.as_str()).collect();
        assert_eq!(order, ["ABC-1", "CL10B104KB8NNNC", "GRM188R71H104KA93D", ""]);
        // two rows share part number C-100N but stay separate
        assert_eq!(r.rows.iter().filter(|x| x.part_number == "C-100N").count(), 2);
        assert_eq!(r.rows[1].quantity, Some(0));
    }

    #[test]
    fn mpn_only_rows_are_imported() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("mpn.csv");
        std::fs::write(&p, "Manufacturer Part Number,Quantity\nSTM32F103C8T6,4\n").unwrap();
        let r = parse_file(&p).unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0].mpn, "STM32F103C8T6");
        assert_eq!(r.rows[0].quantity, Some(4));
    }

    #[test]
    fn template_is_importable() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("mau.xlsx");
        write_template(&p).unwrap();
        let r = parse_file(&p).unwrap();
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.rows[0].quantity, Some(500));
        assert_eq!(r.rows[1].footprint, "LQFP-48");
        assert!(r.unmapped.is_empty());
    }
}
