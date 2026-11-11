//! Text helpers: Vietnamese-aware folding used for header matching and timestamps.

use chrono::{Datelike, NaiveDateTime, Timelike};

/// Lower-case, strip Vietnamese diacritics, replace punctuation with spaces and
/// collapse whitespace. `"Số lượng (pcs)"` becomes `"so luong pcs"`.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = true;
    for ch in s.to_lowercase().chars() {
        // Combining marks (NFD input) are dropped outright.
        if ('\u{0300}'..='\u{036F}').contains(&ch) {
            continue;
        }
        let base = match ch {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ'
            | 'ặ' | 'ẳ' | 'ẵ' => 'a',
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => 'e',
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ'
            | 'ợ' | 'ở' | 'ỡ' => 'o',
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => 'u',
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
            'đ' => 'd',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        };
        if base == ' ' {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(base);
            last_space = false;
        }
    }
    out.trim_end().to_string()
}

/// Parse the frontend timestamp format (`YYYY-MM-DDTHH:MM:SS`, optional fraction).
pub fn parse_ts(ts: &str) -> Option<NaiveDateTime> {
    let core = ts.trim().trim_end_matches('Z');
    let core = core.split('.').next().unwrap_or(core);
    NaiveDateTime::parse_from_str(core, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(core, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(core, "%Y-%m-%dT%H:%M"))
        .ok()
}

/// Vietnamese weekday name.
pub fn weekday_vi(dt: &NaiveDateTime) -> &'static str {
    match dt.weekday() {
        chrono::Weekday::Mon => "Thứ hai",
        chrono::Weekday::Tue => "Thứ ba",
        chrono::Weekday::Wed => "Thứ tư",
        chrono::Weekday::Thu => "Thứ năm",
        chrono::Weekday::Fri => "Thứ sáu",
        chrono::Weekday::Sat => "Thứ bảy",
        chrono::Weekday::Sun => "Chủ nhật",
    }
}

/// `(ngày dd/mm/yyyy, thứ, giờ HH:MM:SS)`; falls back to the raw string when unparsable.
pub fn split_ts(ts: &str) -> (String, String, String) {
    match parse_ts(ts) {
        Some(dt) => (
            format!("{:02}/{:02}/{:04}", dt.day(), dt.month(), dt.year()),
            weekday_vi(&dt).to_string(),
            format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second()),
        ),
        None => (ts.to_string(), String::new(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_vietnamese() {
        assert_eq!(fold("Số lượng"), "so luong");
        assert_eq!(fold("  Vị trí (kệ) "), "vi tri ke");
        assert_eq!(fold("Manufacturer Part Number"), "manufacturer part number");
        assert_eq!(fold("ĐƠN VỊ"), "don vi");
        // NFD form (e + combining acute) folds the same as NFC
        assert_eq!(fold("Te\u{0301}n"), "ten");
    }

    #[test]
    fn splits_timestamp() {
        let (d, w, t) = split_ts("2026-11-11T14:03:05");
        assert_eq!((d.as_str(), w.as_str(), t.as_str()), ("11/11/2026", "Thứ tư", "14:03:05"));
        let (d, w, t) = split_ts("garbage");
        assert_eq!((d.as_str(), w.as_str(), t.as_str()), ("garbage", "", ""));
    }
}
