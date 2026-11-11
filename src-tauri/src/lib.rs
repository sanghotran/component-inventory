//! Tauri shell: wires the pure-Rust core to IPC commands and native dialogs.

use inventory_core::{self as core, ImportResult, Inventory};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// Where `inventory.json` lives (per-user app data folder).
struct DataPath(PathBuf);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

const EXCEL_EXTS: &[&str] = &["xlsx", "xls", "xlsm", "csv", "tsv", "txt"];

async fn pick_open(app: AppHandle, filter_name: &'static str, exts: &'static [&'static str]) -> Option<PathBuf> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter(filter_name, exts)
            .blocking_pick_file()
            .and_then(|f| f.into_path().ok())
    })
    .await
    .ok()
    .flatten()
}

async fn pick_save(app: AppHandle, suggested: String, filter_name: &'static str, exts: &'static [&'static str]) -> Option<PathBuf> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(&suggested)
            .add_filter(filter_name, exts)
            .blocking_save_file()
            .and_then(|f| f.into_path().ok())
    })
    .await
    .ok()
    .flatten()
}

/// Make sure a user-typed save name ends with the wanted extension.
fn with_ext(mut p: PathBuf, ext: &str) -> PathBuf {
    let has = p.extension().map(|e| e.to_string_lossy().eq_ignore_ascii_case(ext)).unwrap_or(false);
    if !has {
        let mut name = p.file_name().map(|s| s.to_os_string()).unwrap_or_default();
        name.push(format!(".{ext}"));
        p.set_file_name(name);
    }
    p
}

#[tauri::command]
fn load_inventory(state: State<DataPath>) -> Result<Inventory, String> {
    core::load(&state.0).map_err(err)
}

#[tauri::command]
fn save_inventory(state: State<DataPath>, inventory: Inventory) -> Result<(), String> {
    core::save(&state.0, &inventory).map_err(err)
}

#[tauri::command]
fn data_file_path(state: State<DataPath>) -> String {
    state.0.display().to_string()
}

/// Open-file dialog, then parse. `None` when the user cancelled.
#[tauri::command]
async fn pick_import_file(app: AppHandle) -> Result<Option<ImportResult>, String> {
    let Some(path) = pick_open(app, "Excel / CSV", EXCEL_EXTS).await else {
        return Ok(None);
    };
    tauri::async_runtime::spawn_blocking(move || core::parse_file(&path))
        .await
        .map_err(err)?
        .map(Some)
        .map_err(err)
}

/// Save-file dialog, then write the two-sheet report. Returns the saved path.
#[tauri::command]
async fn export_report(app: AppHandle, inventory: Inventory, suggested_name: String) -> Result<Option<String>, String> {
    let Some(path) = pick_save(app, suggested_name, "Excel", &["xlsx"]).await else {
        return Ok(None);
    };
    let path = with_ext(path, "xlsx");
    let shown = path.display().to_string();
    tauri::async_runtime::spawn_blocking(move || core::write_report(&path, &inventory))
        .await
        .map_err(err)?
        .map_err(err)?;
    Ok(Some(shown))
}

#[tauri::command]
async fn export_template(app: AppHandle) -> Result<Option<String>, String> {
    let Some(path) = pick_save(app, "Mau_import_linh_kien.xlsx".into(), "Excel", &["xlsx"]).await else {
        return Ok(None);
    };
    let path = with_ext(path, "xlsx");
    let shown = path.display().to_string();
    tauri::async_runtime::spawn_blocking(move || core::write_template(&path))
        .await
        .map_err(err)?
        .map_err(err)?;
    Ok(Some(shown))
}

#[tauri::command]
async fn backup_inventory(app: AppHandle, inventory: Inventory, suggested_name: String) -> Result<Option<String>, String> {
    let Some(path) = pick_save(app, suggested_name, "JSON", &["json"]).await else {
        return Ok(None);
    };
    let path = with_ext(path, "json");
    core::store::write_backup(&path, &inventory).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

/// Pick a backup file and parse it. The frontend asks for confirmation and then
/// calls `save_inventory` itself, so nothing is overwritten here.
#[tauri::command]
async fn restore_inventory(app: AppHandle) -> Result<Option<Inventory>, String> {
    let Some(path) = pick_open(app, "JSON", &["json"]).await else {
        return Ok(None);
    };
    let bytes = std::fs::read(&path).map_err(err)?;
    core::store::from_json(&bytes).map(Some).map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(DataPath(dir.join("inventory.json")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_inventory,
            save_inventory,
            data_file_path,
            pick_import_file,
            export_report,
            export_template,
            backup_inventory,
            restore_inventory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
