//! Sao lưu và khôi phục: một file JSON gồm cài đặt và mọi profile.
//! Nhập vào cũng nhận được một file profile lẻ (`browser.json`) để chia sẻ từng bộ lệnh.

use serde::{Deserialize, Serialize};

use crate::profiles::Profile;
use crate::settings::Settings;

pub const KIND: &str = "radial-menu-backup";
pub const VERSION: u32 = 1;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupOut<'a> {
    kind: &'static str,
    version: u32,
    settings: &'a Settings,
    profiles: &'a [Profile],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupIn {
    #[serde(default)]
    settings: Option<Settings>,
    #[serde(default)]
    profiles: Vec<Profile>,
}

/// Nội dung đọc được từ một file nhập vào, đã kiểm tra hợp lệ.
#[derive(Debug)]
pub struct Parsed {
    pub settings: Option<Settings>,
    pub profiles: Vec<Profile>,
}

pub fn to_json(settings: &Settings, profiles: &[Profile]) -> String {
    let out = BackupOut {
        kind: KIND,
        version: VERSION,
        settings,
        profiles,
    };
    serde_json::to_string_pretty(&out).expect("bản sao lưu luôn ghi được ra JSON") + "\n"
}

pub fn parse(text: &str) -> Result<Parsed, String> {
    let value: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("File không phải JSON hợp lệ: {e}"))?;

    let parsed = if value.get("kind").and_then(|k| k.as_str()) == Some(KIND)
        || value.get("profiles").is_some()
    {
        let b: BackupIn =
            serde_json::from_value(value).map_err(|e| format!("Bản sao lưu bị hỏng: {e}"))?;
        Parsed {
            settings: b.settings.map(Settings::sanitized),
            profiles: b.profiles,
        }
    } else if value.get("id").is_some() && value.get("slots").is_some() {
        let p: Profile =
            serde_json::from_value(value).map_err(|e| format!("File profile bị hỏng: {e}"))?;
        Parsed {
            settings: None,
            profiles: vec![p],
        }
    } else {
        return Err("File này không phải bản sao lưu hay profile của Radial Menu".into());
    };

    let mut profiles = Vec::with_capacity(parsed.profiles.len());
    for p in parsed.profiles {
        let label = if p.name.trim().is_empty() {
            p.id.clone()
        } else {
            p.name.clone()
        };
        let p = p
            .validated()
            .map_err(|e| format!("Profile \"{label}\": {e}"))?;
        if profiles.iter().any(|q: &Profile| q.id == p.id) {
            return Err(format!("Có hai profile cùng mã \"{}\"", p.id));
        }
        profiles.push(p);
    }
    if profiles.is_empty() && parsed.settings.is_none() {
        return Err("File không có profile hay cài đặt nào".into());
    }
    Ok(Parsed {
        settings: parsed.settings,
        profiles,
    })
}

/// Tên file sao lưu theo giờ máy: `radial-menu-sao-luu-2026-10-25-0930.json`.
pub fn file_name() -> String {
    let (y, mo, d, h, mi) = local_time();
    format!("radial-menu-sao-luu-{y:04}-{mo:02}-{d:02}-{h:02}{mi:02}.json")
}

#[cfg(windows)]
fn local_time() -> (u32, u32, u32, u32, u32) {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (
        t.wYear.into(),
        t.wMonth.into(),
        t.wDay.into(),
        t.wHour.into(),
        t.wMinute.into(),
    )
}

#[cfg(not(windows))]
fn local_time() -> (u32, u32, u32, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (y, m, d) = civil(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    (y as u32, m, d, (s / 3600) as u32, (s % 3600 / 60) as u32)
}

/// Ngày dương lịch từ số ngày kể từ 1970-01-01 (thuật toán của Howard Hinnant).
#[cfg(any(test, not(windows)))]
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::Store;

    #[test]
    fn round_trip() {
        let settings = Settings {
            threshold_px: 14.0,
            tone: "light".into(),
            ..Settings::default()
        };
        let store = Store::builtin();
        let text = to_json(&settings, &store.profiles);
        let parsed = parse(&text).unwrap();
        assert_eq!(parsed.profiles.len(), store.profiles.len());
        let s = parsed.settings.unwrap();
        assert_eq!((s.threshold_px, s.tone.as_str()), (14.0, "light"));
    }

    #[test]
    fn single_profile_file() {
        let p = Store::builtin()
            .profiles
            .into_iter()
            .find(|p| p.id == "browser")
            .unwrap();
        let parsed = parse(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(parsed.settings.is_none());
        assert_eq!(parsed.profiles[0].id, "browser");
    }

    #[test]
    fn rejects_bad_files() {
        assert!(parse("không phải json").unwrap_err().contains("JSON"));
        assert!(parse(r#"{"hello": 1}"#)
            .unwrap_err()
            .contains("không phải bản sao lưu"));
        let bad = r#"{"kind":"radial-menu-backup","profiles":[{"id":"x","name":"X","match":{"process":["x.exe"]},
            "slots":[{"dir":"N","label":"A","action":{"type":"hotkey","keys":["Hyper"]}}]}]}"#;
        let err = parse(bad).unwrap_err();
        assert!(
            err.contains("Profile \"X\"") && err.contains("Hyper"),
            "{err}"
        );
        assert!(parse(r#"{"kind":"radial-menu-backup","profiles":[]}"#).is_err());
    }

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_386), (2025, 10, 25));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }
}
