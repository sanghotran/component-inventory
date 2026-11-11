//! Excel report (stock + movement log) and the import template.

use crate::model::{kind_label, Inventory};
use crate::text::split_ts;
use crate::Result;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet};
use std::path::Path;

pub const STOCK_HEADERS: [&str; 11] = [
    "STT", "Part number", "Name", "Footprint", "Pitch", "Quantity", "Descriptions",
    "Manufacturer", "Manufacturer Part Number", "Product Category", "Vị trí",
];
const STOCK_WIDTHS: [f64; 11] = [6.0, 18.0, 28.0, 14.0, 10.0, 10.0, 40.0, 18.0, 26.0, 20.0, 14.0];

pub const HISTORY_HEADERS: [&str; 11] = [
    "STT", "Ngày", "Thứ", "Giờ", "Thao tác", "Part number", "Name", "Số lượng", "Tồn sau", "Người", "Ghi chú",
];
const HISTORY_WIDTHS: [f64; 11] = [6.0, 12.0, 10.0, 10.0, 12.0, 18.0, 28.0, 10.0, 10.0, 18.0, 40.0];

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

    // --- Sheet 1: stock ---
    let ws = wb.add_worksheet();
    ws.set_name("Ton kho")?;
    write_header(ws, &STOCK_HEADERS, &STOCK_WIDTHS)?;
    for (i, p) in inv.parts.iter().enumerate() {
        let r = (i + 1) as u32;
        ws.write_number_with_format(r, 0, (i + 1) as f64, &num)?;
        ws.write_string_with_format(r, 1, &p.part_number, &cell)?;
        ws.write_string_with_format(r, 2, &p.name, &cell)?;
        ws.write_string_with_format(r, 3, &p.footprint, &cell)?;
        ws.write_string_with_format(r, 4, &p.pitch, &cell)?;
        ws.write_number_with_format(r, 5, p.quantity as f64, &num)?;
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
        ws.write_string_with_format(r, 6, &h.name, &cell)?;
        ws.write_number_with_format(r, 7, h.delta as f64, &signed)?;
        ws.write_number_with_format(r, 8, h.after as f64, &num)?;
        ws.write_string_with_format(r, 9, &h.person, &cell)?;
        ws.write_string_with_format(r, 10, &h.note, &cell)?;
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
            part_number: "R-10K".into(), name: "Điện trở 10k".into(), delta: -20, after: 1500, person: "Sang".into(), note: "".into(),
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
        assert_eq!(row[7], "-20");
        assert_eq!(row[9], "Sang");
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
