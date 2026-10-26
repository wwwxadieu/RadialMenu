//! Icon thật của app dưới con trỏ, hiện ở giữa menu. Lấy từ file exe (nét ở 64 px),
//! không được thì hỏi chính cửa sổ. Lưu đệm theo đường dẫn exe nên chỉ đọc một lần.

#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

/// Cạnh icon theo px vật lý: đủ nét cho màn hình 200%.
pub const SIZE: u32 = 64;
/// Số app nhớ icon tối đa; quá thì xoá hết làm lại (hiếm khi chạm tới).
const CACHE_LIMIT: usize = 128;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppIcon {
    pub width: u32,
    pub height: u32,
    /// Điểm ảnh RGBA, mã hoá base64.
    pub data: String,
}

/// Lớp cửa sổ của Desktop: icon của explorer.exe (thư mục) dễ gây nhầm, nên giữ icon profile.
pub fn wants_icon(class: &str, path: &str) -> bool {
    !path.is_empty() && !matches!(class, "Progman" | "WorkerW")
}

#[derive(Default)]
pub struct IconCache {
    map: Mutex<HashMap<String, Option<AppIcon>>>,
}

impl IconCache {
    /// `Some(icon)` = đã thử (icon có thể là `None` nếu app không có icon), `None` = chưa thử.
    pub fn get(&self, path: &str) -> Option<Option<AppIcon>> {
        self.map.lock().unwrap().get(&path.to_lowercase()).cloned()
    }

    pub fn put(&self, path: &str, icon: Option<AppIcon>) {
        let mut map = self.map.lock().unwrap();
        if map.len() >= CACHE_LIMIT {
            map.clear();
        }
        map.insert(path.to_lowercase(), icon);
    }

    /// Lấy từ bộ đệm, chưa có thì đọc rồi nhớ lại.
    pub fn load(&self, path: &str, hwnd: isize) -> Option<AppIcon> {
        if let Some(hit) = self.get(path) {
            return hit;
        }
        let icon = load(path, hwnd);
        self.put(path, icon.clone());
        icon
    }
}

#[cfg(windows)]
fn load(path: &str, hwnd: isize) -> Option<AppIcon> {
    let (bgra, mask, width, height) = win::load(path, hwnd, SIZE)?;
    let rgba = to_rgba(&bgra, mask.as_deref());
    Some(AppIcon {
        width,
        height,
        data: crate::backdrop::base64(&rgba),
    })
}

#[cfg(not(windows))]
fn load(_path: &str, _hwnd: isize) -> Option<AppIcon> {
    None
}

/// BGRA của GDI sang RGBA. Icon kiểu cũ không có kênh alpha thì lấy độ trong suốt
/// từ mask (điểm trắng trong mask = trong suốt).
pub fn to_rgba(bgra: &[u8], mask: Option<&[u8]>) -> Vec<u8> {
    let has_alpha = bgra.chunks_exact(4).any(|p| p[3] != 0);
    let mut out = Vec::with_capacity(bgra.len());
    for (i, p) in bgra.chunks_exact(4).enumerate() {
        let alpha = if has_alpha {
            p[3]
        } else {
            match mask.and_then(|m| m.get(i * 4)) {
                Some(&m) if m != 0 => 0,
                _ => 255,
            }
        };
        out.extend_from_slice(&[p[2], p[1], p[0], alpha]);
    }
    out
}

#[cfg(windows)]
mod win;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_alpha_and_swaps_channels() {
        let bgra = [10, 20, 30, 128, 1, 2, 3, 0];
        assert_eq!(to_rgba(&bgra, None), vec![30, 20, 10, 128, 3, 2, 1, 0]);
    }

    #[test]
    fn old_icons_use_mask() {
        let bgra = [10, 20, 30, 0, 1, 2, 3, 0];
        let mask = [0, 0, 0, 0, 255, 255, 255, 0];
        assert_eq!(
            to_rgba(&bgra, Some(&mask)),
            vec![30, 20, 10, 255, 3, 2, 1, 0]
        );
    }

    #[test]
    fn desktop_keeps_profile_icon() {
        assert!(!wants_icon("Progman", r"C:\Windows\explorer.exe"));
        assert!(!wants_icon("CabinetWClass", ""));
        assert!(wants_icon("Notepad", r"C:\Windows\notepad.exe"));
    }

    #[test]
    fn cache_remembers_misses() {
        let cache = IconCache::default();
        assert!(cache.get("x.exe").is_none());
        cache.put("X.exe", None);
        assert_eq!(cache.get("x.exe"), Some(None));
    }
}
