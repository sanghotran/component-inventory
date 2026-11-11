//! JSON persistence with atomic replace and a one-step `.bak` copy.

use crate::model::Inventory;
use crate::{CoreError, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Load the inventory. A missing file yields an empty inventory; a corrupt
/// file is an error (the file is left untouched so it can be recovered).
pub fn load(path: &Path) -> Result<Inventory> {
    if !path.exists() {
        return Ok(Inventory::default());
    }
    let bytes = fs::read(path)?;
    if bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(Inventory::default());
    }
    let inv: Inventory = serde_json::from_slice(&bytes)?;
    Ok(inv)
}

/// Parse an inventory from JSON bytes (used for restore-from-backup).
pub fn from_json(bytes: &[u8]) -> Result<Inventory> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let looks_right = value.get("parts").map_or(false, |p| p.is_array());
    if !looks_right {
        return Err(CoreError::Message(
            "File sao lưu không đúng định dạng (không có danh sách linh kiện)".into(),
        ));
    }
    let inv: Inventory = serde_json::from_value(value)?;
    Ok(inv)
}

/// Save atomically: write `<file>.tmp`, copy the previous file to `<file>.bak`,
/// then rename the temp file over the target.
pub fn save(path: &Path, inv: &Inventory) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = sibling(path, "tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        serde_json::to_writer_pretty(&mut f, inv)?;
        f.write_all(b"\n")?;
        f.sync_all()?;
    }
    if path.exists() {
        let bak = sibling(path, "bak");
        fs::copy(path, &bak)?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Write a plain backup copy (no tmp/bak dance) to a user-chosen location.
pub fn write_backup(path: &Path, inv: &Inventory) -> Result<()> {
    let mut f = fs::File::create(path)?;
    serde_json::to_writer_pretty(&mut f, inv)?;
    f.write_all(b"\n")?;
    Ok(())
}

fn sibling(path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut name = path.file_name().map(|s| s.to_os_string()).unwrap_or_default();
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Part;

    #[test]
    fn roundtrip_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("inventory.json");
        assert_eq!(load(&file).unwrap(), Inventory::default());

        let mut inv = Inventory::default();
        inv.parts.push(Part { id: "1".into(), part_number: "R-10K".into(), quantity: 5, ..Default::default() });
        save(&file, &inv).unwrap();
        assert_eq!(load(&file).unwrap(), inv);
        assert!(!dir.path().join("inventory.json.bak").exists());

        inv.parts[0].quantity = 3;
        save(&file, &inv).unwrap();
        assert_eq!(load(&file).unwrap().parts[0].quantity, 3);
        let bak = load(&dir.path().join("inventory.json.bak")).unwrap();
        assert_eq!(bak.parts[0].quantity, 5);
        assert!(!dir.path().join("inventory.json.tmp").exists());
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_wipe() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("inventory.json");
        fs::write(&file, b"{not json").unwrap();
        assert!(load(&file).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"{not json");
    }

    #[test]
    fn old_files_without_new_fields_still_load() {
        let json = br#"{"version":1,"parts":[{"id":"a","part_number":"X","quantity":2}],"history":[]}"#;
        let inv = from_json(json).unwrap();
        assert_eq!(inv.parts[0].pitch, "");
        assert_eq!(inv.parts[0].quantity, 2);
        assert!(from_json(b"{}").is_err());
    }
}
