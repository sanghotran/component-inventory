use serde::{Deserialize, Serialize};

/// One stock-keeping line. Field names are the JSON keys the frontend uses too.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Part {
    pub id: String,
    pub part_number: String,
    pub name: String,
    pub footprint: String,
    pub pitch: String,
    pub quantity: i64,
    pub description: String,
    pub manufacturer: String,
    /// Manufacturer Part Number
    pub mpn: String,
    /// Product Category
    pub category: String,
    /// Vị trí (storage location)
    pub location: String,
}

/// One line of the movement log.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct HistoryEntry {
    pub id: String,
    /// Local wall-clock time, `YYYY-MM-DDTHH:MM:SS` (no timezone).
    pub ts: String,
    /// take | add | create | edit | delete | import | restore
    pub kind: String,
    pub part_id: String,
    pub part_number: String,
    pub name: String,
    /// Signed quantity change (negative when taking out).
    pub delta: i64,
    /// Quantity on hand after this entry.
    pub after: i64,
    pub person: String,
    pub note: String,
}

/// The whole persisted document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Inventory {
    pub version: u32,
    pub parts: Vec<Part>,
    pub history: Vec<HistoryEntry>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self { version: 1, parts: Vec::new(), history: Vec::new() }
    }
}

/// Vietnamese label for a history `kind`.
pub fn kind_label(kind: &str) -> &str {
    match kind {
        "take" => "Lấy ra",
        "add" => "Nhập thêm",
        "create" => "Tạo mới",
        "edit" => "Sửa",
        "delete" => "Xóa",
        "import" => "Import",
        "restore" => "Khôi phục",
        other => other,
    }
}
