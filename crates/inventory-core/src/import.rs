//! Import parts from Excel (.xlsx/.xlsm/.xls/.ods) or CSV/TSV.
//!
//! The header row is located automatically (title rows above it are fine) and
//! headers are matched against Vietnamese/English synonyms after folding, so
//! "Số lượng", "SO LUONG", "Qty" and "Quantity" all map to `quantity`.

use crate::text::fold;
use crate::{CoreError, Result};
use calamine::{open_workbook_auto, Data, Reader};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    PartNumber,
    Name,
    Footprint,
    Pitch,
    Quantity,
    Description,
    Manufacturer,
    Mpn,
    Category,
    Location,
}

/// Checked in this order, so the more specific "manufacturer part number"
/// wins over both "manufacturer" and "part number".
const SYNONYMS: &[(Field, &[&str])] = &[
    (Field::Mpn, &[
        "manufacturer part number", "manufacturer part no", "manufacturer part", "mfr part number",
        "mfr part no", "mfr part", "mfg part number", "mfg part no", "mfg pn", "mfr pn", "mfr p n",
        "mpn", "ma nha san xuat", "ma nsx", "part number nsx", "ma hang san xuat",
    ]),
    (Field::PartNumber, &[
        "part number", "part no", "partnumber", "part num", "part", "p n", "pn", "ma linh kien",
        "ma", "ma so", "ma sp", "ma hang", "ma noi bo", "sku", "code", "item code", "internal part number",
    ]),
    (Field::Manufacturer, &[
        "manufacturer", "mfr", "mfg", "brand", "hang", "hang sx", "hang san xuat", "nha san xuat",
        "nsx", "vendor", "manufacturer name",
    ]),
    (Field::Quantity, &[
        "quantity", "qty", "so luong", "sl", "ton", "ton kho", "stock", "in stock", "qty in stock",
        "count", "amount", "so luong ton",
    ]),
    (Field::Pitch, &["pitch", "buoc chan", "khoang cach chan", "pitch mm"]),
    (Field::Footprint, &[
        "footprint", "package", "package case", "case", "kieu chan", "dang chan", "dong goi", "kieu dong goi",
    ]),
    (Field::Description, &[
        "description", "descriptions", "desc", "mo ta", "chi tiet", "thong so", "ghi chu", "note", "notes",
    ]),
    (Field::Category, &[
        "product category", "category", "danh muc", "loai", "phan loai", "nhom", "type", "group",
        "loai linh kien", "nhom linh kien",
    ]),
    (Field::Location, &[
        "vi tri", "location", "position", "bin", "ngan", "ke", "hop", "o", "vi tri luu", "storage",
        "storage location", "place", "noi luu", "vi tri kho", "ngan keo",
    ]),
    (Field::Name, &[
        "name", "ten", "ten linh kien", "component", "component name", "linh kien", "item", "item name",
        "ten hang", "product", "product name", "part name", "ten san pham",
    ]),
];

/// Map a raw header cell to a field, or `None` when unrecognised.
pub fn match_header(header: &str) -> Option<Field> {
    let h = fold(header);
    if h.is_empty() {
        return None;
    }
    for (field, syns) in SYNONYMS {
        if syns.iter().any(|s| *s == h) {
            return Some(*field);
        }
    }
    // Looser pass: a long synonym contained in the header, e.g. "Quantity (pcs)".
    for (field, syns) in SYNONYMS {
        if syns.iter().any(|s| s.len() >= 4 && h.contains(s)) {
            return Some(*field);
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnMapping {
    pub field: Field,
    pub header: String,
    /// 0-based column index in the sheet
    pub column: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImportedRow {
    /// 1-based row number in the sheet (for error messages)
    pub row: usize,
    pub part_number: String,
    pub name: String,
    pub footprint: String,
    pub pitch: String,
    /// `None` when the cell is empty or not a number
    pub quantity: Option<i64>,
    pub description: String,
    pub manufacturer: String,
    pub mpn: String,
    pub category: String,
    pub location: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImportResult {
    pub file_name: String,
    pub sheet: String,
    /// 1-based header row number
    pub header_row: usize,
    pub mapping: Vec<ColumnMapping>,
    pub unmapped: Vec<String>,
    pub rows: Vec<ImportedRow>,
    /// Rows under the header that had no part number, name or MPN
    pub skipped_rows: usize,
    /// Rows whose quantity cell was non-empty but not a number
    pub bad_quantity_rows: usize,
}

/// Parse a lenient quantity: "12", "12.0", "1,000", "1 000", " 7 pcs".
pub fn parse_quantity(raw: &str) -> Option<i64> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    // Keep digits, sign, separators; stop at the first letter (e.g. "7 pcs").
    let mut cleaned = String::new();
    for ch in s.chars() {
        match ch {
            '0'..='9' | '-' | '+' | '.' | ',' => cleaned.push(ch),
            ' ' | '\u{a0}' | '_' | '\'' => {}
            _ => break,
        }
    }
    if cleaned.is_empty() {
        return None;
    }
    // Quantities are whole numbers, so a separator followed by exactly three
    // digits is a thousands separator ("1.000" = "1,000" = 1000). Anything else
    // is a decimal point and gets rounded ("12.0" -> 12, "2,5" -> 3).
    let last_sep = cleaned.rfind(|c| c == '.' || c == ',');
    let normalized = match last_sep {
        None => cleaned.clone(),
        Some(i) => {
            let tail = &cleaned[i + 1..];
            let mixed = cleaned.contains('.') && cleaned.contains(',');
            let seps = cleaned.matches(|c| c == '.' || c == ',').count();
            let thousands = tail.len() == 3 && tail.chars().all(|c| c.is_ascii_digit()) && (!mixed || seps > 1);
            if thousands && !mixed {
                cleaned.replace(['.', ','], "")
            } else {
                let (head, dec) = cleaned.split_at(i);
                format!("{}.{}", head.replace(['.', ','], ""), &dec[1..])
            }
        }
    };
    normalized.parse::<f64>().ok().filter(|v| v.is_finite()).map(|v| v.round() as i64)
}

fn cell_to_string(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.as_f64().to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#{e:?}"),
    }
}

/// A sheet reduced to strings.
type Grid = Vec<Vec<String>>;

/// Find the best header row within the first 30 rows. Returns (row index, mapping).
fn detect_header(grid: &Grid) -> Option<(usize, Vec<ColumnMapping>)> {
    let mut best: Option<(usize, Vec<ColumnMapping>)> = None;
    for (ri, row) in grid.iter().take(30).enumerate() {
        let mut mapping: Vec<ColumnMapping> = Vec::new();
        for (ci, cell) in row.iter().enumerate() {
            if let Some(field) = match_header(cell) {
                if !mapping.iter().any(|m| m.field == field) {
                    mapping.push(ColumnMapping { field, header: cell.clone(), column: ci });
                }
            }
        }
        let has_key = mapping.iter().any(|m| matches!(m.field, Field::PartNumber | Field::Name | Field::Mpn));
        if mapping.len() >= 2 && has_key {
            let better = best.as_ref().map_or(true, |(_, b)| mapping.len() > b.len());
            if better {
                best = Some((ri, mapping));
            }
        }
    }
    best
}

fn grid_to_result(grid: &Grid, file_name: &str, sheet: &str) -> Option<ImportResult> {
    let (hr, mapping) = detect_header(grid)?;
    let header_cells = &grid[hr];
    let unmapped: Vec<String> = header_cells
        .iter()
        .enumerate()
        .filter(|(ci, h)| !h.trim().is_empty() && !mapping.iter().any(|m| m.column == *ci))
        .map(|(_, h)| h.clone())
        .collect();

    let col = |f: Field| mapping.iter().find(|m| m.field == f).map(|m| m.column);
    let get = |row: &Vec<String>, f: Field| -> String {
        col(f).and_then(|c| row.get(c)).cloned().unwrap_or_default()
    };

    let mut rows = Vec::new();
    let mut skipped = 0usize;
    let mut bad_qty = 0usize;
    for (ri, row) in grid.iter().enumerate().skip(hr + 1) {
        let part_number = get(row, Field::PartNumber);
        let name = get(row, Field::Name);
        let mpn = get(row, Field::Mpn);
        if part_number.is_empty() && name.is_empty() && mpn.is_empty() {
            skipped += 1;
            continue;
        }
        let qty_raw = get(row, Field::Quantity);
        let quantity = parse_quantity(&qty_raw);
        if quantity.is_none() && !qty_raw.trim().is_empty() {
            bad_qty += 1;
        }
        rows.push(ImportedRow {
            row: ri + 1,
            part_number,
            name,
            footprint: get(row, Field::Footprint),
            pitch: get(row, Field::Pitch),
            quantity,
            description: get(row, Field::Description),
            manufacturer: get(row, Field::Manufacturer),
            mpn,
            category: get(row, Field::Category),
            location: get(row, Field::Location),
        });
    }
    Some(ImportResult {
        file_name: file_name.to_string(),
        sheet: sheet.to_string(),
        header_row: hr + 1,
        mapping,
        unmapped,
        rows,
        skipped_rows: skipped,
        bad_quantity_rows: bad_qty,
    })
}

fn no_header_error() -> CoreError {
    CoreError::Message(
        "Không tìm thấy dòng tiêu đề. File cần có ít nhất cột Part number (hoặc Name) và một cột khác như Quantity, Footprint…".into(),
    )
}

fn read_excel(path: &Path, file_name: &str) -> Result<ImportResult> {
    let mut wb = open_workbook_auto(path)?;
    let names = wb.sheet_names().to_vec();
    let mut best: Option<ImportResult> = None;
    for name in names {
        let range = match wb.worksheet_range(&name) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let grid: Grid = range.rows().map(|r| r.iter().map(cell_to_string).collect()).collect();
        if let Some(res) = grid_to_result(&grid, file_name, &name) {
            let better = best.as_ref().map_or(true, |b| res.mapping.len() > b.mapping.len());
            if better {
                best = Some(res);
            }
            // First sheet with a good header and real rows is normally the right one.
            if best.as_ref().map_or(false, |b| !b.rows.is_empty()) {
                break;
            }
        }
    }
    best.ok_or_else(no_header_error)
}

fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            // UTF-16 with BOM (Excel "Unicode Text"), else legacy Vietnamese code page.
            if bytes.starts_with(b"\xFF\xFE") || bytes.starts_with(b"\xFE\xFF") {
                let (s, _, _) = encoding_rs::UTF_16LE.decode(bytes);
                return s.into_owned();
            }
            let (s, _, _) = encoding_rs::WINDOWS_1258.decode(bytes);
            s.into_owned()
        }
    }
}

fn sniff_delimiter(text: &str) -> u8 {
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let candidates = [(b'\t', first.matches('\t').count()), (b';', first.matches(';').count()), (b',', first.matches(',').count())];
    candidates.iter().max_by_key(|(_, n)| *n).map(|(d, n)| if *n == 0 { b',' } else { *d }).unwrap_or(b',')
}

fn read_csv(path: &Path, file_name: &str) -> Result<ImportResult> {
    let bytes = std::fs::read(path)?;
    let text = decode_text(&bytes);
    let delim = sniff_delimiter(&text);
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delim)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let mut grid: Grid = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        grid.push(rec.iter().map(|s| s.to_string()).collect());
    }
    grid_to_result(&grid, file_name, "CSV").ok_or_else(no_header_error)
}

/// Parse an Excel or CSV file chosen by the user.
pub fn parse_file(path: &Path) -> Result<ImportResult> {
    let file_name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "csv" | "tsv" | "txt" => read_csv(path, &file_name),
        "xlsx" | "xlsm" | "xlsb" | "xls" | "ods" => read_excel(path, &file_name),
        _ => Err(CoreError::Message(format!("Không hỗ trợ định dạng .{ext}. Hãy dùng .xlsx, .xls hoặc .csv"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_synonyms() {
        assert_eq!(match_header("Part number"), Some(Field::PartNumber));
        assert_eq!(match_header("Manufacturer Part Number"), Some(Field::Mpn));
        assert_eq!(match_header("Mfr. Part #"), Some(Field::Mpn));
        assert_eq!(match_header("Manufacturer"), Some(Field::Manufacturer));
        assert_eq!(match_header("Số lượng"), Some(Field::Quantity));
        assert_eq!(match_header("Quantity (pcs)"), Some(Field::Quantity));
        assert_eq!(match_header("Vị trí"), Some(Field::Location));
        assert_eq!(match_header("Product Category"), Some(Field::Category));
        assert_eq!(match_header("Descriptions"), Some(Field::Description));
        assert_eq!(match_header("Pitch"), Some(Field::Pitch));
        assert_eq!(match_header("Tên linh kiện"), Some(Field::Name));
        assert_eq!(match_header("Footprint"), Some(Field::Footprint));
        assert_eq!(match_header("Giá"), None);
        assert_eq!(match_header(""), None);
    }

    #[test]
    fn quantities() {
        assert_eq!(parse_quantity("12"), Some(12));
        assert_eq!(parse_quantity("12.0"), Some(12));
        assert_eq!(parse_quantity("1,000"), Some(1000));
        assert_eq!(parse_quantity("1.000"), Some(1000));
        assert_eq!(parse_quantity("1.000.000"), Some(1_000_000));
        assert_eq!(parse_quantity("2,5"), Some(3));
        assert_eq!(parse_quantity("1,234.5"), Some(1235));
        assert_eq!(parse_quantity("1.234,5"), Some(1235));
        assert_eq!(parse_quantity(" 7 pcs"), Some(7));
        assert_eq!(parse_quantity("-3"), Some(-3));
        assert_eq!(parse_quantity(""), None);
        assert_eq!(parse_quantity("n/a"), None);
    }

    #[test]
    fn csv_with_title_rows_and_vietnamese_headers() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("kho.csv");
        std::fs::write(
            &p,
            "\u{feff}BẢNG TỒN KHO LINH KIỆN;;;;\nNgày: 11/11/2026;;;;\nPart number;Tên;Số lượng;Vị trí;Pitch;Giá\nR-10K;Điện trở 10k;1.000;Kệ A1;;100\n;;;;;\nC-100N;Tụ 100nF;abc;Kệ A2;0.5mm;\n",
        )
        .unwrap();
        let r = parse_file(&p).unwrap();
        assert_eq!(r.header_row, 3);
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.skipped_rows, 1);
        assert_eq!(r.bad_quantity_rows, 1);
        assert_eq!(r.unmapped, vec!["Giá".to_string()]);
        assert_eq!(r.rows[0].part_number, "R-10K");
        assert_eq!(r.rows[0].name, "Điện trở 10k");
        assert_eq!(r.rows[0].quantity, Some(1000));
        assert_eq!(r.rows[0].location, "Kệ A1");
        assert_eq!(r.rows[1].quantity, None);
        assert_eq!(r.rows[1].pitch, "0.5mm");
        assert_eq!(r.rows[1].row, 6);
    }

    #[test]
    fn csv_without_header_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.csv");
        std::fs::write(&p, "a,b,c\n1,2,3\n").unwrap();
        let err = parse_file(&p).unwrap_err().to_string();
        assert!(err.contains("tiêu đề"), "{err}");
    }

    #[test]
    fn legacy_encoding_csv() {
        // Excel "CSV" on a Vietnamese Windows box writes code page 1258, where
        // tone marks are separate combining bytes (0xD2 = acute, 0xF2 = dot below).
        let bytes: &[u8] = b"Part number,T\xEAn,S\xF4\xD2 l\xFD\xF5\xF2ng\nX1,\xD4\xD2c v\xEDt,5\n";
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("legacy.csv");
        std::fs::write(&p, bytes).unwrap();
        let r = parse_file(&p).unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0].quantity, Some(5));
        assert_eq!(fold(&r.rows[0].name), "oc vit");
    }

    #[test]
    fn unsupported_extension() {
        let err = parse_file(Path::new("foo.pdf")).unwrap_err().to_string();
        assert!(err.contains(".pdf"));
    }
}
