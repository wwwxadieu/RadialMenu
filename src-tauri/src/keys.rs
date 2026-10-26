//! Đọc tên phím trong profile ("Ctrl", "Shift", "T", "Left", "F12", "`") thành mã
//! phím ảo của Windows. Thuần Rust để test được; mã VK là các số cố định của Windows.

#![cfg_attr(not(windows), allow(dead_code))]

pub const VK_BACK: u16 = 0x08;
pub const VK_TAB: u16 = 0x09;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;
pub const VK_PAUSE: u16 = 0x13;
pub const VK_CAPITAL: u16 = 0x14;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
pub const VK_PRIOR: u16 = 0x21;
pub const VK_NEXT: u16 = 0x22;
pub const VK_END: u16 = 0x23;
pub const VK_HOME: u16 = 0x24;
pub const VK_LEFT: u16 = 0x25;
pub const VK_UP: u16 = 0x26;
pub const VK_RIGHT: u16 = 0x27;
pub const VK_DOWN: u16 = 0x28;
pub const VK_SNAPSHOT: u16 = 0x2C;
pub const VK_INSERT: u16 = 0x2D;
pub const VK_DELETE: u16 = 0x2E;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_APPS: u16 = 0x5D;
pub const VK_F1: u16 = 0x70;
pub const VK_NUMLOCK: u16 = 0x90;
pub const VK_SCROLL: u16 = 0x91;
pub const VK_VOLUME_MUTE: u16 = 0xAD;
pub const VK_VOLUME_DOWN: u16 = 0xAE;
pub const VK_VOLUME_UP: u16 = 0xAF;
pub const VK_MEDIA_NEXT_TRACK: u16 = 0xB0;
pub const VK_MEDIA_PREV_TRACK: u16 = 0xB1;
pub const VK_MEDIA_PLAY_PAUSE: u16 = 0xB3;

/// Một phím cần nhấn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    pub vk: u16,
    /// Phím mở rộng (mũi tên, Insert/Delete, Home/End…): phải gắn cờ extended
    /// để Windows không hiểu nhầm thành phím số.
    pub extended: bool,
    /// Với phím ký tự: trên bàn phím US có cần giữ Shift không.
    pub shift: bool,
    /// Ký tự gốc, để trên Windows tra lại theo bố cục bàn phím đang dùng.
    pub ch: Option<char>,
}

const fn named(vk: u16, extended: bool) -> Key {
    Key {
        vk,
        extended,
        shift: false,
        ch: None,
    }
}

/// Phím ký tự theo bố cục US: (ký tự, VK, cần Shift).
const US_CHARS: &[(char, u16, bool)] = &[
    (';', 0xBA, false),
    (':', 0xBA, true),
    ('=', 0xBB, false),
    ('+', 0xBB, true),
    (',', 0xBC, false),
    ('<', 0xBC, true),
    ('-', 0xBD, false),
    ('_', 0xBD, true),
    ('.', 0xBE, false),
    ('>', 0xBE, true),
    ('/', 0xBF, false),
    ('?', 0xBF, true),
    ('`', 0xC0, false),
    ('~', 0xC0, true),
    ('[', 0xDB, false),
    ('{', 0xDB, true),
    ('\\', 0xDC, false),
    ('|', 0xDC, true),
    (']', 0xDD, false),
    ('}', 0xDD, true),
    ('\'', 0xDE, false),
    ('"', 0xDE, true),
    ('!', 0x31, true),
    ('@', 0x32, true),
    ('#', 0x33, true),
    ('$', 0x34, true),
    ('%', 0x35, true),
    ('^', 0x36, true),
    ('&', 0x37, true),
    ('*', 0x38, true),
    ('(', 0x39, true),
    (')', 0x30, true),
];

/// Đọc một tên phím. Không phân biệt hoa thường.
pub fn parse_key(name: &str) -> Result<Key, String> {
    let raw = name.trim();
    if raw.is_empty() {
        return Err("tên phím trống".into());
    }

    let mut chars = raw.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return char_key(c).ok_or_else(|| format!("không biết phím \"{raw}\""));
    }

    let lower = raw.to_lowercase();
    let key = match lower.as_str() {
        "ctrl" | "control" => named(VK_CONTROL, false),
        "shift" => named(VK_SHIFT, false),
        "alt" => named(VK_MENU, false),
        "win" | "windows" | "meta" | "super" | "cmd" => named(VK_LWIN, true),
        "enter" | "return" => named(VK_RETURN, false),
        "tab" => named(VK_TAB, false),
        "esc" | "escape" => named(VK_ESCAPE, false),
        "space" | "spacebar" => named(VK_SPACE, false),
        "backspace" | "back" => named(VK_BACK, false),
        "delete" | "del" => named(VK_DELETE, true),
        "insert" | "ins" => named(VK_INSERT, true),
        "home" => named(VK_HOME, true),
        "end" => named(VK_END, true),
        "pageup" | "pgup" => named(VK_PRIOR, true),
        "pagedown" | "pgdn" => named(VK_NEXT, true),
        "up" => named(VK_UP, true),
        "down" => named(VK_DOWN, true),
        "left" => named(VK_LEFT, true),
        "right" => named(VK_RIGHT, true),
        "printscreen" | "prtsc" => named(VK_SNAPSHOT, true),
        "apps" | "menu" | "contextmenu" => named(VK_APPS, true),
        "capslock" => named(VK_CAPITAL, false),
        "numlock" => named(VK_NUMLOCK, true),
        "scrolllock" => named(VK_SCROLL, false),
        "pause" => named(VK_PAUSE, false),
        "volumemute" | "mute" => named(VK_VOLUME_MUTE, true),
        "volumedown" => named(VK_VOLUME_DOWN, true),
        "volumeup" => named(VK_VOLUME_UP, true),
        "mediaplay" | "playpause" => named(VK_MEDIA_PLAY_PAUSE, true),
        "medianext" | "nexttrack" => named(VK_MEDIA_NEXT_TRACK, true),
        "mediaprev" | "prevtrack" => named(VK_MEDIA_PREV_TRACK, true),
        _ => match lower.strip_prefix('f').and_then(|n| n.parse::<u16>().ok()) {
            Some(n @ 1..=24) => named(VK_F1 + n - 1, false),
            _ => return Err(format!("không biết phím \"{raw}\"")),
        },
    };
    Ok(key)
}

fn char_key(c: char) -> Option<Key> {
    match c {
        '←' => Some(named(VK_LEFT, true)),
        '→' => Some(named(VK_RIGHT, true)),
        '↑' => Some(named(VK_UP, true)),
        '↓' => Some(named(VK_DOWN, true)),
        'a'..='z' | 'A'..='Z' => Some(named(c.to_ascii_uppercase() as u16, false)),
        '0'..='9' => Some(named(c as u16, false)),
        ' ' => Some(named(VK_SPACE, false)),
        _ => US_CHARS
            .iter()
            .find(|(ch, _, _)| *ch == c)
            .map(|&(ch, vk, shift)| Key {
                vk,
                extended: false,
                shift,
                ch: Some(ch),
            }),
    }
}

/// Đọc cả tổ hợp phím, giữ nguyên thứ tự: nhấn lần lượt, nhả theo thứ tự ngược lại.
pub fn parse_chord(names: &[String]) -> Result<Vec<Key>, String> {
    if names.is_empty() {
        return Err("tổ hợp phím trống".into());
    }
    names.iter().map(|n| parse_key(n)).collect()
}

/// Tên phím đẹp để hiện trên menu: "left" → "←", "ctrl" → "Ctrl".
pub fn pretty(name: &str) -> String {
    let raw = name.trim();
    match raw.to_lowercase().as_str() {
        "left" => "←".into(),
        "right" => "→".into(),
        "up" => "↑".into(),
        "down" => "↓".into(),
        "ctrl" | "control" => "Ctrl".into(),
        "shift" => "Shift".into(),
        "alt" => "Alt".into(),
        "win" | "windows" | "meta" | "super" | "cmd" => "Win".into(),
        "enter" | "return" => "Enter".into(),
        "esc" | "escape" => "Esc".into(),
        "delete" | "del" => "Delete".into(),
        "space" | "spacebar" => "Space".into(),
        "tab" => "Tab".into(),
        lower if lower.len() == 1 => raw.to_uppercase(),
        lower if lower.starts_with('f') && lower[1..].parse::<u8>().is_ok() => raw.to_uppercase(),
        _ => raw.to_string(),
    }
}

pub fn pretty_chord(names: &[String]) -> String {
    names
        .iter()
        .map(|n| pretty(n))
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn modifiers_and_letters() {
        let keys = parse_chord(&chord(&["Ctrl", "Shift", "t"])).unwrap();
        assert_eq!(
            keys.iter().map(|k| k.vk).collect::<Vec<_>>(),
            vec![VK_CONTROL, VK_SHIFT, 0x54]
        );
        assert!(!keys[2].extended);
    }

    #[test]
    fn navigation_keys_are_extended() {
        for name in [
            "Left", "right", "UP", "down", "Delete", "Home", "End", "PageUp", "pgdn", "Insert",
            "Win",
        ] {
            assert!(parse_key(name).unwrap().extended, "{name}");
        }
        assert!(parse_key("←").unwrap().extended);
    }

    #[test]
    fn function_keys() {
        assert_eq!(parse_key("F2").unwrap().vk, 0x71);
        assert_eq!(parse_key("f12").unwrap().vk, 0x7B);
        assert_eq!(parse_key("F24").unwrap().vk, 0x87);
        assert!(parse_key("F25").is_err());
        assert!(parse_key("F0").is_err());
    }

    #[test]
    fn punctuation_us_layout() {
        let k = parse_key("`").unwrap();
        assert_eq!((k.vk, k.shift, k.ch), (0xC0, false, Some('`')));
        assert_eq!(parse_key("\\").unwrap().vk, 0xDC);
        assert_eq!(parse_key("/").unwrap().vk, 0xBF);
        assert_eq!(parse_key(".").unwrap().vk, 0xBE);
        let plus = parse_key("+").unwrap();
        assert_eq!((plus.vk, plus.shift), (0xBB, true));
    }

    #[test]
    fn unknown_and_empty() {
        assert!(parse_key("Hyper").is_err());
        assert!(parse_key("  ").is_err());
        assert!(parse_chord(&[]).is_err());
        assert!(parse_key("é").is_err());
    }

    #[test]
    fn pretty_names() {
        assert_eq!(pretty_chord(&chord(&["alt", "Left"])), "Alt+←");
        assert_eq!(
            pretty_chord(&chord(&["Ctrl", "shift", "t"])),
            "Ctrl+Shift+T"
        );
        assert_eq!(pretty_chord(&chord(&["win", "."])), "Win+.");
        assert_eq!(pretty_chord(&chord(&["f12"])), "F12");
    }
}
