//! Pure-Rust core of the component inventory app.
//!
//! Everything in here is independent of Tauri so it can be unit-tested on any
//! machine (no WebView toolchain needed). The Tauri shell in `src-tauri` only
//! wires these functions to commands and native file dialogs.

pub mod export;
pub mod import;
pub mod model;
pub mod store;
pub mod text;

pub use export::{write_report, write_template};
pub use import::{parse_file, ImportResult, ImportedRow};
pub use model::{HistoryEntry, Inventory, Part};
pub use store::{load, save};

/// Error type shared by the whole core crate.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Lỗi đọc/ghi file: {0}")]
    Io(#[from] std::io::Error),
    #[error("File dữ liệu không hợp lệ: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Không đọc được file Excel: {0}")]
    Excel(#[from] calamine::Error),
    #[error("Không đọc được file CSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("Không ghi được file Excel: {0}")]
    XlsxWrite(#[from] rust_xlsxwriter::XlsxError),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
