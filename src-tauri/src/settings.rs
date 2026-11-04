//! Cài đặt chung, lưu trong `settings.json` cạnh thư mục `profiles`.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::gesture::Config;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Khoảng kéo tối thiểu để mở menu, px logic (4–30).
    pub threshold_px: f64,
    /// Bán kính vùng huỷ ở giữa menu, px logic (10–80).
    pub cancel_radius_px: f64,
    /// Kéo ra quá xa thì menu tự đóng, không chạy lệnh.
    pub close_when_far: bool,
    /// Khoảng cách tự đóng tính từ điểm nhấn, px logic (300–1000).
    pub close_distance_px: f64,
    /// `system` (theo cài đặt hiệu ứng của Windows), `on` hoặc `off`.
    pub animations: String,
    /// Bỏ qua menu khi app dưới con trỏ đang chạy toàn màn hình.
    pub disable_when_fullscreen: bool,
    /// Tên file chương trình không bao giờ mở menu tròn.
    pub exclude: Vec<String>,
    /// `dark` hoặc `light`.
    pub tone: String,
    /// Màu nhấn dạng `#RRGGBB`.
    pub accent: String,
    /// Làm mờ nội dung phía sau menu (chụp vùng màn hình dưới menu).
    pub blur: bool,
    /// Ẩn menu khỏi ảnh chụp màn hình. Tắt đi nếu cần chụp menu để làm tài liệu.
    pub hide_from_capture: bool,
    /// Kích thước menu tròn so với mặc định (0,8–1,4).
    pub menu_scale: f64,
    pub launch_at_login: bool,
    /// Tự hỏi GitHub có bản mới không (lúc khởi động và mỗi ngày). Chỉ báo, không tự cài.
    pub check_updates: bool,
    /// Phiên bản bộ profile mặc định đã ghi ra thư mục cấu hình; dùng để thêm
    /// profile mặc định mới ở bản sau mà không ghi đè file người dùng đã sửa.
    /// File cũ chưa có trường này được hiểu là 0.
    pub profiles_version: u32,
    /// Phiên bản của chính file cài đặt, để chuyển giá trị mặc định cũ sang mới.
    /// File cũ chưa có trường này được hiểu là 0 (không lấy theo `Default`).
    #[serde(default)]
    pub settings_version: u32,
}

/// Màu nhấn có sẵn; màu đầu tiên là mặc định (chàm, cùng màu với logo).
pub const ACCENTS: [&str; 5] = ["#7C83FF", "#60CDFF", "#2DD4BF", "#F5B451", "#F27AA8"];
/// Màu nhấn mặc định của các bản trước bản cài đặt v1.
const OLD_DEFAULT_ACCENT: &str = "#60CDFF";
/// Khoảng cách tự đóng mặc định của bản cài đặt v1 (quá gần, hay đóng nhầm).
const OLD_DEFAULT_CLOSE_DISTANCE: f64 = 300.0;
pub const SETTINGS_VERSION: u32 = 2;

impl Default for Settings {
    fn default() -> Self {
        Self {
            threshold_px: 10.0,
            cancel_radius_px: 30.0,
            close_when_far: true,
            close_distance_px: 450.0,
            animations: "system".into(),
            disable_when_fullscreen: true,
            exclude: vec!["blender.exe".into()],
            tone: "dark".into(),
            accent: ACCENTS[0].into(),
            blur: true,
            hide_from_capture: true,
            menu_scale: 1.0,
            launch_at_login: false,
            check_updates: true,
            profiles_version: 0,
            settings_version: SETTINGS_VERSION,
        }
    }
}

impl Settings {
    /// Đọc `path`; chưa có thì ghi bản mặc định. File hỏng → dùng mặc định và báo lỗi.
    pub fn load(path: &Path) -> (Self, Option<String>) {
        match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(s) => {
                    let (s, migrated) = s.migrated();
                    let s = s.sanitized();
                    let err = if migrated {
                        s.save(path)
                            .err()
                            .map(|e| format!("Không ghi được settings.json: {e}"))
                    } else {
                        None
                    };
                    (s, err)
                }
                Err(e) => (Self::default(), Some(format!("settings.json: {e}"))),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let s = Self::default();
                let err = s
                    .save(path)
                    .err()
                    .map(|e| format!("Không ghi được settings.json: {e}"));
                (s, err)
            }
            Err(e) => (
                Self::default(),
                Some(format!("Không đọc được settings.json: {e}")),
            ),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).expect("settings luôn ghi được ra JSON");
        fs::write(path, text + "\n")
    }

    /// Chuyển file cài đặt của bản cũ: màu nhấn mặc định cũ thành màu mặc định mới.
    /// Trả về `true` nếu có thay đổi cần ghi lại.
    fn migrated(mut self) -> (Self, bool) {
        if self.settings_version >= SETTINGS_VERSION {
            return (self, false);
        }
        if self.settings_version < 1 && self.accent.trim().eq_ignore_ascii_case(OLD_DEFAULT_ACCENT)
        {
            self.accent = ACCENTS[0].into();
        }
        if self.settings_version < 2 && self.close_distance_px == OLD_DEFAULT_CLOSE_DISTANCE {
            self.close_distance_px = Self::default().close_distance_px;
        }
        self.settings_version = SETTINGS_VERSION;
        (self, true)
    }

    /// Đưa mọi trường về giá trị hợp lệ (dùng cả khi đọc file lẫn khi giao diện gửi lên).
    pub fn sanitized(mut self) -> Self {
        let d = Self::default();
        if !self.threshold_px.is_finite() {
            self.threshold_px = d.threshold_px;
        }
        if !self.cancel_radius_px.is_finite() {
            self.cancel_radius_px = d.cancel_radius_px;
        }
        if !self.close_distance_px.is_finite() {
            self.close_distance_px = d.close_distance_px;
        }
        if !self.menu_scale.is_finite() {
            self.menu_scale = d.menu_scale;
        }
        self.menu_scale = ((self.menu_scale * 20.0).round() / 20.0).clamp(0.8, 1.4);
        self.threshold_px = self.threshold_px.round().clamp(4.0, 30.0);
        self.cancel_radius_px = self.cancel_radius_px.round().clamp(10.0, 80.0);
        self.close_distance_px =
            ((self.close_distance_px / 10.0).round() * 10.0).clamp(300.0, 1000.0);

        if !matches!(self.animations.as_str(), "system" | "on" | "off") {
            self.animations = d.animations;
        }
        if !matches!(self.tone.as_str(), "dark" | "light") {
            self.tone = d.tone;
        }
        let accent = self.accent.trim();
        self.accent = if accent.len() == 7
            && accent.starts_with('#')
            && accent[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            accent.to_ascii_uppercase()
        } else {
            d.accent
        };

        let mut exclude: Vec<String> = Vec::new();
        for p in &self.exclude {
            let p = p.trim().to_lowercase();
            if !p.is_empty() && !exclude.contains(&p) {
                exclude.push(p);
            }
        }
        self.exclude = exclude;
        self
    }

    pub fn gesture_config(&self) -> Config {
        Config {
            threshold_px: self.threshold_px,
            cancel_radius_px: self.cancel_radius_px,
            close_distance_px: self.close_when_far.then_some(self.close_distance_px),
            // Mép đĩa menu (172 px ở kích thước 100%), lùi vào một chút.
            drill_radius_px: 165.0 * self.menu_scale,
        }
    }

    pub fn is_excluded(&self, process: &str) -> bool {
        self.exclude
            .iter()
            .any(|p| p.trim().eq_ignore_ascii_case(process))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_file_keeps_defaults_and_clamps() {
        let s: Settings =
            serde_json::from_str(r#"{"thresholdPx": 100, "exclude": ["Game.exe"]}"#).unwrap();
        let s = s.sanitized();
        assert_eq!(s.threshold_px, 30.0);
        assert_eq!(s.cancel_radius_px, 30.0);
        assert!(s.disable_when_fullscreen);
        assert!(
            s.close_when_far,
            "file cũ chưa có trường này thì bật tự đóng"
        );
        assert_eq!(s.gesture_config().close_distance_px, Some(450.0));
        assert!(
            s.blur && s.hide_from_capture,
            "file cũ chưa có hai trường mới thì bật mặc định"
        );
        assert!(s.is_excluded("game.exe"));
        assert!(!s.is_excluded("blender.exe"));
    }

    #[test]
    fn old_default_accent_moves_to_new_default() {
        // File của bản cũ: chưa có settingsVersion, màu nhấn là mặc định cũ.
        let old: Settings = serde_json::from_str(r##"{"accent": "#60cdff"}"##).unwrap();
        let (s, changed) = old.migrated();
        assert!(changed);
        assert_eq!(
            (s.accent.as_str(), s.settings_version),
            (ACCENTS[0], SETTINGS_VERSION)
        );

        // Người dùng tự chọn màu khác thì giữ nguyên.
        let picked: Settings = serde_json::from_str(r##"{"accent": "#FFB454"}"##).unwrap();
        assert_eq!(picked.migrated().0.accent, "#FFB454");

        // Bản v1 lưu khoảng cách tự đóng mặc định cũ (300): chuyển sang mặc định mới.
        let v1: Settings = serde_json::from_str(
            r##"{"settingsVersion": 1, "closeDistancePx": 300, "accent": "#60CDFF"}"##,
        )
        .unwrap();
        let (s, changed) = v1.migrated();
        assert!(changed);
        assert_eq!(s.close_distance_px, 450.0);
        assert_eq!(
            s.accent, OLD_DEFAULT_ACCENT,
            "v1 đã chuyển màu rồi: xanh trời lúc này là chủ ý"
        );
        let picked: Settings =
            serde_json::from_str(r##"{"settingsVersion": 1, "closeDistancePx": 600}"##).unwrap();
        assert_eq!(picked.migrated().0.close_distance_px, 600.0);

        // File mới có settingsVersion: chọn xanh trời là chủ ý, không đổi.
        let chosen = Settings {
            accent: OLD_DEFAULT_ACCENT.into(),
            ..Settings::default()
        };
        let (s, changed) = chosen.migrated();
        assert!(!changed);
        assert_eq!(s.accent, OLD_DEFAULT_ACCENT);
    }

    #[test]
    fn menu_scale_snaps_to_five_percent() {
        let s = Settings {
            menu_scale: 1.234,
            ..Settings::default()
        }
        .sanitized();
        assert!((s.menu_scale - 1.25).abs() < 1e-9);
        assert_eq!(
            Settings {
                menu_scale: 3.0,
                ..Settings::default()
            }
            .sanitized()
            .menu_scale,
            1.4
        );
        assert_eq!(
            Settings {
                menu_scale: 0.1,
                ..Settings::default()
            }
            .sanitized()
            .menu_scale,
            0.8
        );
        let old: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.sanitized().menu_scale, 1.0);
    }

    #[test]
    fn default_excludes_blender() {
        assert!(Settings::default().is_excluded("Blender.exe"));
    }

    #[test]
    fn sanitize_bad_values() {
        let s = Settings {
            tone: "neon".into(),
            animations: "sometimes".into(),
            accent: "blue".into(),
            exclude: vec![" Game.EXE ".into(), "game.exe".into(), "".into()],
            threshold_px: 12.6,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.tone, "dark");
        assert_eq!(s.animations, "system");
        assert_eq!(s.accent, ACCENTS[0]);
        assert_eq!(s.exclude, vec!["game.exe".to_string()]);
        assert_eq!(s.threshold_px, 13.0);
        let ok = Settings {
            accent: "#ffb454".into(),
            tone: "light".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!((ok.accent.as_str(), ok.tone.as_str()), ("#FFB454", "light"));
    }
}
