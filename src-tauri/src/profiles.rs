//! Bộ lệnh cho từng app, lưu thành file JSON trong thư mục cấu hình.
//!
//! Mỗi profile là một file `<id>.json`. Lần chạy đầu (thư mục chưa có file nào)
//! app ghi ra 4 profile mặc định. Profile có id `default` dùng cho mọi app chưa
//! có profile riêng.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::gesture::DIR_NAMES;
use crate::keys;

pub const DEFAULT_ID: &str = "default";

/// 8 hướng theo thứ tự chỉ số của menu: 0 = Bắc, theo chiều kim đồng hồ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirName {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl DirName {
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    /// Gửi tổ hợp phím vào app, ví dụ `["Ctrl", "Shift", "T"]`.
    Hotkey { keys: Vec<String> },
    /// Chạy một chương trình. Hỗ trợ biến môi trường dạng `%USERPROFILE%`.
    Command {
        cmd: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    /// Mở URL hoặc đường dẫn bằng app mặc định của Windows.
    Url { url: String },
    /// Gõ một đoạn văn bản (Unicode, có xuống dòng).
    Text { text: String },
    /// Chạy lần lượt nhiều bước.
    Sequence { steps: Vec<Action> },
    /// Chờ giữa các bước của một chuỗi, tối đa 5 giây.
    Wait { ms: u64 },
    /// Mở cửa sổ cài đặt của Radial Menu.
    Settings,
    /// Mở file đang chọn trong File Explorer hoặc trên Desktop bằng chương trình `app`
    /// (tên trong PATH, tên đã đăng ký như `msedge.exe`, hoặc đường dẫn đầy đủ).
    /// Không có `app` = hiện hộp thoại "Open with" của Windows để chọn.
    OpenWith {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        app: Option<String>,
    },
    /// Menu con: thêm tối đa 8 lệnh nữa, mở bằng cách kéo qua mép menu ở góc này
    /// hoặc nhả chuột ở góc này rồi bấm chọn. Không lồng thêm menu con bên trong.
    Submenu { slots: Vec<Slot> },
}

impl Action {
    /// Mô tả ngắn hiện ở giữa menu khi chỉ vào góc này.
    pub fn describe(&self) -> String {
        match self {
            Action::Hotkey { keys } => keys::pretty_chord(keys),
            Action::Command { cmd, args, .. } => {
                let mut s = cmd.rsplit(['\\', '/']).next().unwrap_or(cmd).to_string();
                for a in args {
                    s.push(' ');
                    s.push_str(a);
                }
                s
            }
            Action::Url { url } => url
                .split_once("://")
                .map_or(url.as_str(), |(_, rest)| rest)
                .trim_end_matches('/')
                .to_string(),
            Action::Text { text } => {
                let first = text.lines().next().unwrap_or("");
                let short: String = first.chars().take(14).collect();
                if short.chars().count() < text.chars().count() {
                    format!("“{short}…”")
                } else {
                    format!("“{short}”")
                }
            }
            Action::Sequence { steps } => steps
                .iter()
                .filter(|s| !matches!(s, Action::Wait { .. }))
                .map(Action::describe)
                .collect::<Vec<_>>()
                .join(" → "),
            Action::Wait { ms } => format!("chờ {ms} ms"),
            Action::Settings => "mở cài đặt".into(),
            Action::OpenWith { app: Some(app) } => {
                let name = app.rsplit(['\\', '/']).next().unwrap_or(app);
                let name = name.strip_suffix(".exe").unwrap_or(name);
                format!("mở bằng {name}")
            }
            Action::OpenWith { app: None } => "chọn app để mở".into(),
            Action::Submenu { slots } => format!("{} lệnh ›", slots.len()),
        }
    }

    /// Là lệnh Dán (Ctrl+V hoặc Shift+Insert).
    pub fn is_paste(&self) -> bool {
        let Action::Hotkey { keys } = self else {
            return false;
        };
        let mut names: Vec<String> = keys
            .iter()
            .map(|k| match k.trim().to_lowercase().as_str() {
                "control" => "ctrl".to_string(),
                "ins" => "insert".to_string(),
                other => other.to_string(),
            })
            .collect();
        names.sort();
        names == ["ctrl", "v"] || names == ["insert", "shift"]
    }

    /// Hành động gửi phím/chữ vào app, nên cần cửa sổ đích đang active.
    pub fn needs_focus(&self) -> bool {
        match self {
            Action::Hotkey { .. } | Action::Text { .. } => true,
            Action::Sequence { steps } => steps.iter().any(Action::needs_focus),
            Action::Command { .. }
            | Action::Url { .. }
            | Action::Wait { .. }
            | Action::Settings
            | Action::OpenWith { .. }
            | Action::Submenu { .. } => false,
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Khi nào một góc dùng được.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum When {
    /// Luôn dùng được.
    Always,
    /// Chỉ khi clipboard đang có dữ liệu (góc Dán).
    Clipboard,
}

/// Tình trạng máy lúc mở menu, để biết góc nào đang dùng được.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    pub clipboard: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub dir: DirName,
    pub label: String,
    #[serde(default)]
    pub icon: String,
    pub action: Action,
    /// Click chuột trái tại điểm nhấn trước khi chạy lệnh, để chọn đúng thứ dưới
    /// con trỏ (file trong Explorer, chữ trong editor).
    #[serde(default, skip_serializing_if = "is_false")]
    pub click_first: bool,
    /// Điều kiện để góc dùng được. Bỏ trống = tự đoán: góc Dán (Ctrl+V,
    /// Shift+Insert) chỉ dùng được khi clipboard có dữ liệu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}

impl Slot {
    pub fn condition(&self) -> When {
        self.when.unwrap_or(if self.action.is_paste() {
            When::Clipboard
        } else {
            When::Always
        })
    }

    /// Lý do góc này đang không dùng được, `None` nếu dùng được.
    pub fn unavailable(&self, ctx: &Context) -> Option<&'static str> {
        match self.condition() {
            When::Clipboard if !ctx.clipboard => Some("Clipboard trống"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Match {
    /// Tên file chương trình, không phân biệt hoa thường: `chrome.exe`.
    #[serde(default)]
    pub process: Vec<String>,
    /// Lớp cửa sổ gốc, không phân biệt hoa thường: `Progman`, `WorkerW` (desktop).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub class: Vec<String>,
}

impl Match {
    /// Mức độ cụ thể khi khớp (càng nhiều điều kiện càng cao), `None` nếu không khớp.
    /// Profile không có điều kiện nào (như `default`) không bao giờ khớp.
    fn score(&self, process: Option<&str>, class: Option<&str>) -> Option<u8> {
        fn hit(list: &[String], value: Option<&str>) -> bool {
            value.is_some_and(|v| list.iter().any(|x| x.trim().eq_ignore_ascii_case(v)))
        }
        if self.process.is_empty() && self.class.is_empty() {
            return None;
        }
        let process_ok = self.process.is_empty() || hit(&self.process, process);
        let class_ok = self.class.is_empty() || hit(&self.class, class);
        (process_ok && class_ok)
            .then(|| u8::from(!self.process.is_empty()) + u8::from(!self.class.is_empty()))
    }
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default, rename = "match")]
    pub matcher: Match,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub slots: Vec<Slot>,
    /// Tên file đã đọc profile này ra (không ghi vào JSON).
    #[serde(skip)]
    pub file: Option<String>,
}

/// Mã profile: chữ thường không dấu, số, `-`, `_`; tối đa 40 ký tự. Dùng làm tên file.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Kiểm tra các góc của một vòng menu (vòng chính hoặc menu con), chuẩn hoá tên và
/// sắp theo hướng. `prefix` thêm vào trước tên góc trong lời báo lỗi.
fn validate_slots(slots: &mut [Slot], in_submenu: bool, prefix: &str) -> Result<(), String> {
    let mut seen = [false; 8];
    for slot in slots.iter_mut() {
        let i = slot.dir.index();
        let name = DIR_NAMES[i];
        if std::mem::replace(&mut seen[i], true) {
            return Err(format!("{prefix}Có hai góc cùng hướng {name}"));
        }
        slot.label = slot.label.trim().to_string();
        if slot.label.is_empty() {
            return Err(format!("{prefix}Góc {name}: cần tên hiển thị"));
        }
        if let Action::Submenu { slots: children } = &mut slot.action {
            if in_submenu {
                return Err(format!("{prefix}Góc {name}: menu con không được lồng nhau"));
            }
            if children.is_empty() {
                return Err(format!("Góc {name}: menu con chưa có lệnh nào"));
            }
            validate_slots(children, true, &format!("Góc {name}, menu con: "))?;
        } else {
            validate_action(&slot.action, 0).map_err(|e| format!("{prefix}Góc {name}: {e}"))?;
        }
    }
    slots.sort_by_key(|s| s.dir.index());
    Ok(())
}

fn validate_action(action: &Action, depth: u8) -> Result<(), String> {
    match action {
        Action::Submenu { .. } => Err("menu con không dùng trong chuỗi bước".into()),
        Action::OpenWith { .. } if depth > 0 => Err("Mở bằng không dùng trong chuỗi bước".into()),
        Action::OpenWith { app: Some(app) } if app.trim().is_empty() => {
            Err("cần nhập chương trình để mở file".into())
        }
        Action::Hotkey { keys: k } => keys::parse_chord(k).map(|_| ()),
        Action::Command { cmd, .. } if cmd.trim().is_empty() => {
            Err("cần nhập lệnh cần chạy".into())
        }
        Action::Url { url } if url.trim().is_empty() => Err("cần nhập địa chỉ".into()),
        Action::Text { text } if text.is_empty() => Err("cần nhập văn bản".into()),
        Action::Sequence { steps } => {
            if depth > 0 {
                return Err("chuỗi bước không được lồng nhau".into());
            }
            if steps.is_empty() {
                return Err("chuỗi bước đang trống".into());
            }
            steps.iter().try_for_each(|s| validate_action(s, depth + 1))
        }
        Action::Wait { ms } if *ms > 5_000 => Err("chờ tối đa 5000 ms".into()),
        Action::Settings if depth > 0 => Err("Mở cài đặt không dùng trong chuỗi bước".into()),
        _ => Ok(()),
    }
}

impl Profile {
    /// Kiểm tra và chuẩn hoá profile trước khi lưu. Lỗi trả về là câu hiện được cho người dùng.
    pub fn validated(mut self) -> Result<Self, String> {
        self.id = self.id.trim().to_string();
        if !is_valid_id(&self.id) {
            return Err("Mã profile chỉ gồm chữ thường không dấu, số, - và _".into());
        }
        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            return Err("Tên profile không được để trống".into());
        }

        let mut processes: Vec<String> = Vec::new();
        for p in &self.matcher.process {
            let p = p.trim().to_lowercase();
            if !p.is_empty() && !processes.contains(&p) {
                processes.push(p);
            }
        }
        let mut classes: Vec<String> = Vec::new();
        for c in &self.matcher.class {
            let c = c.trim().to_string();
            if !c.is_empty() && !classes.iter().any(|x| x.eq_ignore_ascii_case(&c)) {
                classes.push(c);
            }
        }
        self.matcher = Match {
            process: processes,
            class: classes,
        };
        if self.id == DEFAULT_ID {
            // Profile Mặc định dùng cho mọi app còn lại, không khớp theo tên.
            self.matcher = Match::default();
            self.enabled = true;
        } else if self.matcher.process.is_empty() && self.matcher.class.is_empty() {
            return Err("Cần ít nhất một tên chương trình, ví dụ notepad.exe".into());
        }

        validate_slots(&mut self.slots, false, "")?;
        Ok(self)
    }
    pub fn slot(&self, dir: usize) -> Option<&Slot> {
        find_slot(&self.slots, dir)
    }

    /// Các góc có menu con (bit `1 << hướng`), báo cho hook để kéo qua mép là mở.
    pub fn submenu_mask(&self) -> u8 {
        self.slots
            .iter()
            .filter(|s| matches!(s.action, Action::Submenu { .. }))
            .fold(0, |m, s| m | (1 << s.dir.index()))
    }

    /// Góc `dir` của menu con nằm ở góc `parent`.
    pub fn sub_slot(&self, parent: usize, dir: usize) -> Option<&Slot> {
        match &self.slot(parent)?.action {
            Action::Submenu { slots } => find_slot(slots, dir),
            _ => None,
        }
    }

    /// Dữ liệu gửi sang trang menu.
    pub fn view(&self, process: Option<&str>, ctx: &Context) -> MenuProfile {
        MenuProfile {
            id: self.id.clone(),
            name: self.name.clone(),
            process: process.unwrap_or("không rõ app").to_string(),
            icon: self.icon.clone(),
            slots: ring_view(&self.slots, ctx),
        }
    }

    /// Dữ liệu menu con của góc `parent`, để trang menu thay vòng lệnh tại chỗ.
    pub fn sub_view(&self, parent: usize, ctx: &Context) -> Option<MenuProfile> {
        let slot = self.slot(parent)?;
        let Action::Submenu { slots } = &slot.action else {
            return None;
        };
        Some(MenuProfile {
            id: format!("{}/{}", self.id, DIR_NAMES[parent]),
            name: slot.label.clone(),
            process: self.name.clone(),
            icon: slot.icon.clone(),
            slots: ring_view(slots, ctx),
        })
    }
}

fn find_slot(slots: &[Slot], dir: usize) -> Option<&Slot> {
    slots.iter().find(|s| s.dir.index() == dir)
}

fn ring_view(slots: &[Slot], ctx: &Context) -> Vec<Option<MenuSlot>> {
    (0..8)
        .map(|i| {
            find_slot(slots, i).map(|s| MenuSlot {
                label: s.label.clone(),
                keys: s.action.describe(),
                icon: s.icon.clone(),
                unavailable: s.unavailable(ctx),
                submenu: matches!(s.action, Action::Submenu { .. }),
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuSlot {
    pub label: String,
    pub keys: String,
    pub icon: String,
    /// Lý do góc đang tắt (hiện mờ, không chọn được).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<&'static str>,
    /// Góc mở menu con.
    #[serde(skip_serializing_if = "is_false")]
    pub submenu: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuProfile {
    pub id: String,
    pub name: String,
    pub process: String,
    pub icon: String,
    /// Đúng 8 phần tử; `null` = góc trống.
    pub slots: Vec<Option<MenuSlot>>,
}

#[derive(Debug, Clone)]
pub struct Store {
    pub profiles: Vec<Profile>,
}

impl Store {
    pub fn builtin() -> Self {
        Self {
            profiles: defaults(),
        }
    }

    /// Profile cho cửa sổ (process, lớp cửa sổ). Nhiều profile cùng khớp thì lấy
    /// profile cụ thể nhất (Desktop thắng Explorer); không khớp gì thì dùng `default`.
    pub fn resolve(&self, process: Option<&str>, class: Option<&str>) -> &Profile {
        let mut best: Option<(u8, &Profile)> = None;
        for p in self.profiles.iter().filter(|p| p.enabled) {
            if let Some(score) = p.matcher.score(process, class) {
                if best.is_none_or(|(s, _)| score > s) {
                    best = Some((score, p));
                }
            }
        }
        if let Some((_, p)) = best {
            return p;
        }
        self.profiles
            .iter()
            .find(|p| p.id == DEFAULT_ID)
            .or_else(|| self.profiles.last())
            .expect("Store luôn có ít nhất profile default")
    }

    /// Đọc mọi file `*.json` trong `dir`. Thư mục chưa có file nào thì ghi bộ mặc định.
    /// Trả về cả danh sách lỗi (file hỏng bị bỏ qua, không làm app dừng).
    pub fn load_dir(dir: &Path) -> (Self, Vec<String>) {
        let mut errors = Vec::new();
        if let Err(e) = fs::create_dir_all(dir) {
            errors.push(format!("Không tạo được thư mục {}: {e}", dir.display()));
            return (Self::builtin(), errors);
        }

        let mut files: Vec<_> = match fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .is_some_and(|x| x.eq_ignore_ascii_case("json"))
                })
                .collect(),
            Err(e) => {
                errors.push(format!("Không đọc được thư mục {}: {e}", dir.display()));
                return (Self::builtin(), errors);
            }
        };

        if files.is_empty() {
            if let Err(e) = write_defaults(dir) {
                errors.push(format!("Không ghi được profile mặc định: {e}"));
            }
            return (Self::builtin(), errors);
        }

        files.sort();
        let mut profiles = Vec::new();
        for path in files {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            match fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str::<Profile>(&text).map_err(|e| e.to_string()))
            {
                Ok(mut p) => {
                    p.file = Some(name);
                    profiles.push(p);
                }
                Err(e) => errors.push(format!("{name}: {e}")),
            }
        }
        if !profiles.iter().any(|p| p.id == DEFAULT_ID) {
            profiles.push(default_profile());
        }
        (Self { profiles }, errors)
    }
}

/// Lưu một profile đã sửa. `original_id` = mã trước khi sửa (`None` = profile mới).
/// Đổi mã thì file cũ được thay bằng file mới.
pub fn save_profile(
    dir: &Path,
    store: &Store,
    profile: Profile,
    original_id: Option<&str>,
) -> Result<Profile, String> {
    let profile = profile.validated()?;
    if original_id == Some(DEFAULT_ID) && profile.id != DEFAULT_ID {
        return Err("Không đổi được mã của profile Mặc định".into());
    }
    if original_id != Some(profile.id.as_str()) && store.profiles.iter().any(|p| p.id == profile.id)
    {
        return Err(format!("Đã có profile mã \"{}\"", profile.id));
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    write_profile(dir, &profile).map_err(|e| format!("Không ghi được {}.json: {e}", profile.id))?;

    let new_file = format!("{}.json", profile.id);
    let old_file = original_id
        .and_then(|id| store.profiles.iter().find(|p| p.id == id))
        .and_then(|p| p.file.clone());
    if let Some(old) = old_file.filter(|f| !f.eq_ignore_ascii_case(&new_file)) {
        let _ = fs::remove_file(dir.join(old));
    }
    Ok(profile)
}

/// Xoá file của một profile (không xoá được profile Mặc định).
pub fn delete_profile(dir: &Path, store: &Store, id: &str) -> Result<(), String> {
    if id == DEFAULT_ID {
        return Err("Không xoá được profile Mặc định".into());
    }
    let profile = store
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or("Không tìm thấy profile")?;
    let file = profile.file.clone().unwrap_or_else(|| format!("{id}.json"));
    match fs::remove_file(dir.join(&file)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Không xoá được {file}: {e}")),
    }
}

pub fn write_defaults(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    for p in defaults() {
        write_profile(dir, &p)?;
    }
    Ok(())
}

fn write_profile(dir: &Path, p: &Profile) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(p).expect("profile luôn ghi được ra JSON");
    fs::write(dir.join(format!("{}.json", p.id)), text + "\n")
}

/// Phiên bản hiện tại của bộ profile mặc định.
pub const PROFILES_VERSION: u32 = 3;

/// Góc Đông Nam của Desktop: mở cài đặt Radial Menu (thay cho Mở Terminal từ bộ mặc định v3).
fn settings_slot() -> Slot {
    slot(
        DirName::SE,
        "Cài đặt Radial Menu",
        "settings",
        Action::Settings,
    )
}

/// Bộ mặc định v3: góc Mở Terminal của Desktop thành Cài đặt Radial Menu,
/// nhưng chỉ khi người dùng chưa sửa góc đó.
fn patch_desktop_settings(dir: &Path) -> Result<(), String> {
    let path = dir.join("desktop.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let mut p: Profile = serde_json::from_str(&text).map_err(|e| format!("desktop.json: {e}"))?;
    let old_terminal = Action::Command {
        cmd: "wt".into(),
        args: vec!["-d".into(), "%DESKTOP%".into()],
        cwd: None,
    };
    let Some(se) = p.slots.iter_mut().find(|s| s.dir == DirName::SE) else {
        return Ok(());
    };
    if se.label != "Mở Terminal" || se.action != old_terminal {
        return Ok(());
    }
    *se = settings_slot();
    write_profile(dir, &p).map_err(|e| format!("Không ghi được desktop.json: {e}"))
}

/// Profile mặc định và phiên bản bộ mặc định đã có nó.
fn builtin_since() -> Vec<(u32, Profile)> {
    vec![
        (1, browser_profile()),
        (1, code_profile()),
        (1, default_profile()),
        (2, desktop_profile()),
        (1, explorer_profile()),
    ]
}

/// Ghi những profile mặc định mới hơn `from_version` vào `dir` nếu chưa có file trùng tên.
/// Thư mục trống thì ghi cả bộ. Không bao giờ ghi đè file đã có.
/// Trả về phiên bản mới để lưu lại vào settings.json.
pub fn migrate(dir: &Path, from_version: u32) -> (u32, Vec<String>) {
    let mut errors = Vec::new();
    if let Err(e) = fs::create_dir_all(dir) {
        errors.push(format!("Không tạo được thư mục {}: {e}", dir.display()));
        return (from_version, errors);
    }
    let empty = fs::read_dir(dir)
        .map(|mut it| it.next().is_none())
        .unwrap_or(true);
    for (since, p) in builtin_since() {
        if !empty && since <= from_version {
            continue;
        }
        let path = dir.join(format!("{}.json", p.id));
        if path.exists() {
            continue;
        }
        if let Err(e) = write_profile(dir, &p) {
            errors.push(format!("Không ghi được {}.json: {e}", p.id));
        }
    }
    if !empty && from_version < 3 {
        if let Err(e) = patch_desktop_settings(dir) {
            errors.push(e);
        }
    }
    (PROFILES_VERSION.max(from_version), errors)
}

// ---------- 4 profile mặc định (khớp bảng trong kế hoạch) ----------

fn keys_of(k: &[&str]) -> Action {
    Action::Hotkey {
        keys: k.iter().map(|s| s.to_string()).collect(),
    }
}

fn slot(dir: DirName, label: &str, icon: &str, action: Action) -> Slot {
    Slot {
        dir,
        label: label.into(),
        icon: icon.into(),
        action,
        click_first: false,
        when: None,
    }
}

fn slot_click(dir: DirName, label: &str, icon: &str, action: Action) -> Slot {
    Slot {
        click_first: true,
        ..slot(dir, label, icon, action)
    }
}

fn procs(list: &[&str]) -> Match {
    Match {
        process: list.iter().map(|s| s.to_string()).collect(),
        class: Vec::new(),
    }
}

fn browser_profile() -> Profile {
    use DirName::*;
    Profile {
        id: "browser".into(),
        name: "Trình duyệt".into(),
        icon: "globe".into(),
        matcher: procs(&[
            "chrome.exe",
            "msedge.exe",
            "firefox.exe",
            "brave.exe",
            "opera.exe",
            "vivaldi.exe",
        ]),
        enabled: true,
        slots: vec![
            slot(N, "Tab mới", "plus", keys_of(&["Ctrl", "T"])),
            slot(NE, "Tải lại", "refresh", keys_of(&["Ctrl", "R"])),
            slot(E, "Tiến", "arrowRight", keys_of(&["Alt", "Right"])),
            slot(SE, "Tìm trong trang", "search", keys_of(&["Ctrl", "F"])),
            slot(S, "Đóng tab", "x", keys_of(&["Ctrl", "W"])),
            slot(SW, "Mở lại tab", "undo", keys_of(&["Ctrl", "Shift", "T"])),
            slot(W, "Quay lại", "arrowLeft", keys_of(&["Alt", "Left"])),
            slot(
                NW,
                "Tab ẩn danh",
                "eyeOff",
                keys_of(&["Ctrl", "Shift", "N"]),
            ),
        ],
        file: None,
    }
}

fn explorer_profile() -> Profile {
    use DirName::*;
    // Gõ lệnh vào thanh địa chỉ: Explorer chạy nó ngay trong thư mục đang mở.
    let terminal = Action::Sequence {
        steps: vec![
            keys_of(&["Alt", "D"]),
            Action::Wait { ms: 150 },
            Action::Text {
                text: "wt -d .".into(),
            },
            Action::Wait { ms: 50 },
            keys_of(&["Enter"]),
        ],
    };
    Profile {
        id: "explorer".into(),
        name: "File Explorer".into(),
        icon: "folder".into(),
        matcher: procs(&["explorer.exe"]),
        enabled: true,
        slots: vec![
            slot(
                N,
                "Thư mục mới",
                "folderPlus",
                keys_of(&["Ctrl", "Shift", "N"]),
            ),
            slot_click(NE, "Đổi tên", "pencil", keys_of(&["F2"])),
            slot(E, "Dán", "paste", keys_of(&["Ctrl", "V"])),
            slot(SE, "Mở Terminal", "terminal", terminal),
            slot_click(S, "Xoá", "trash", keys_of(&["Delete"])),
            slot_click(SW, "Thuộc tính", "info", keys_of(&["Alt", "Enter"])),
            slot_click(W, "Sao chép", "copy", keys_of(&["Ctrl", "C"])),
            slot_click(
                NW,
                "Copy đường dẫn",
                "pathCopy",
                keys_of(&["Ctrl", "Shift", "C"]),
            ),
        ],
        file: None,
    }
}

/// Desktop cũng thuộc explorer.exe nhưng khác lớp cửa sổ: thay Đổi tên bằng Làm mới.
fn desktop_profile() -> Profile {
    use DirName::*;
    let mut p = explorer_profile();
    p.id = "desktop".into();
    p.name = "Desktop".into();
    p.icon = "monitor".into();
    p.matcher = Match {
        process: vec!["explorer.exe".into()],
        class: vec!["Progman".into(), "WorkerW".into()],
    };
    for s in &mut p.slots {
        match s.dir {
            // Click trước để desktop nhận phím F5 chắc chắn.
            NE => *s = slot_click(NE, "Làm mới", "refresh", keys_of(&["F5"])),
            // Ngoài desktop là chỗ tự nhiên để mở cài đặt của chính Radial Menu.
            SE => *s = settings_slot(),
            _ => {}
        }
    }
    p
}

fn code_profile() -> Profile {
    use DirName::*;
    Profile {
        id: "code".into(),
        name: "Code editor".into(),
        icon: "code".into(),
        matcher: procs(&[
            "code.exe",
            "code - insiders.exe",
            "cursor.exe",
            "windsurf.exe",
            "vscodium.exe",
        ]),
        enabled: true,
        slots: vec![
            slot(N, "Định dạng", "format", keys_of(&["Shift", "Alt", "F"])),
            slot_click(NE, "Tới định nghĩa", "goto", keys_of(&["F12"])),
            slot(E, "Mở file nhanh", "search", keys_of(&["Ctrl", "P"])),
            slot(SE, "Terminal", "terminal", keys_of(&["Ctrl", "`"])),
            slot(S, "Comment", "comment", keys_of(&["Ctrl", "/"])),
            slot_click(SW, "Đổi tên biến", "textCursor", keys_of(&["F2"])),
            slot(W, "Chạy", "play", keys_of(&["F5"])),
            slot(NW, "Chia đôi", "columns", keys_of(&["Ctrl", "\\"])),
        ],
        file: None,
    }
}

fn default_profile() -> Profile {
    use DirName::*;
    Profile {
        id: DEFAULT_ID.into(),
        name: "Mặc định".into(),
        icon: "grid".into(),
        matcher: Match::default(),
        enabled: true,
        slots: vec![
            slot(N, "Chụp vùng", "crop", keys_of(&["Win", "Shift", "S"])),
            slot(NE, "Clipboard", "clipboard", keys_of(&["Win", "V"])),
            slot(E, "Cửa sổ kế", "windows", keys_of(&["Alt", "Tab"])),
            slot(SE, "Hiện Desktop", "monitor", keys_of(&["Win", "D"])),
            slot(S, "Thu nhỏ", "minimize", keys_of(&["Win", "Down"])),
            // Win+L không gửi giả lập được (Windows chặn), nên gọi thẳng API khoá máy.
            slot(
                SW,
                "Khoá máy",
                "lock",
                Action::Command {
                    cmd: "rundll32.exe".into(),
                    args: vec!["user32.dll,LockWorkStation".into()],
                    cwd: None,
                },
            ),
            slot(W, "Emoji", "smile", keys_of(&["Win", "."])),
            slot(NW, "Tìm kiếm", "search", keys_of(&["Win", "S"])),
        ],
        file: None,
    }
}

pub fn defaults() -> Vec<Profile> {
    builtin_since().into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_window_submenu() -> Profile {
        let mut p = default_profile();
        let children = vec![
            slot(
                DirName::N,
                " Phóng to ",
                "maximize",
                keys_of(&["Win", "Up"]),
            ),
            slot(DirName::S, "Thu nhỏ", "minimize", keys_of(&["Win", "Down"])),
        ];
        p.slots[2] = slot(
            DirName::E,
            "Cửa sổ",
            "windows",
            Action::Submenu { slots: children },
        );
        p
    }

    #[test]
    fn submenu_round_trips_and_validates() {
        let p = with_window_submenu().validated().unwrap();
        assert_eq!(p.submenu_mask(), 1 << 2);
        assert_eq!(
            p.sub_slot(2, 0).unwrap().label,
            "Phóng to",
            "tên trong menu con cũng được cắt khoảng trắng"
        );
        assert!(p.sub_slot(2, 2).is_none());
        assert!(p.sub_slot(0, 0).is_none(), "góc không có menu con");

        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""type":"submenu""#));
        let back: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);

        let ctx = Context { clipboard: true };
        let view = p.view(None, &ctx);
        assert!(view.slots[2].as_ref().unwrap().submenu);
        assert_eq!(view.slots[2].as_ref().unwrap().keys, "2 lệnh ›");
        let sub = p.sub_view(2, &ctx).unwrap();
        assert_eq!(sub.name, "Cửa sổ");
        assert_eq!(sub.slots[4].as_ref().unwrap().label, "Thu nhỏ");
        assert!(p.sub_view(0, &ctx).is_none());
    }

    #[test]
    fn open_with_json_describe_and_rules() {
        let app: Action =
            serde_json::from_str(r#"{"type":"openWith","app":"C:\\Apps\\Code.exe"}"#).unwrap();
        assert_eq!(app.describe(), "mở bằng Code");
        let dialog: Action = serde_json::from_str(r#"{"type":"openWith"}"#).unwrap();
        assert_eq!(dialog, Action::OpenWith { app: None });
        assert_eq!(
            serde_json::to_string(&dialog).unwrap(),
            r#"{"type":"openWith"}"#
        );
        assert_eq!(dialog.describe(), "chọn app để mở");
        assert!(!app.needs_focus());

        assert!(validate_action(&app, 0).is_ok());
        assert!(validate_action(&dialog, 0).is_ok());
        assert!(validate_action(
            &Action::OpenWith {
                app: Some(" ".into())
            },
            0
        )
        .is_err());
        let in_sequence = Action::Sequence {
            steps: vec![dialog],
        };
        assert!(validate_action(&in_sequence, 0).is_err());
    }

    #[test]
    fn submenu_rules() {
        let mut empty = with_window_submenu();
        empty.slots[2].action = Action::Submenu { slots: vec![] };
        assert_eq!(
            empty.validated().unwrap_err(),
            "Góc Đông: menu con chưa có lệnh nào"
        );

        let mut nested = with_window_submenu();
        if let Action::Submenu { slots } = &mut nested.slots[2].action {
            slots.push(slot(
                DirName::W,
                "Thêm",
                "plus",
                Action::Submenu { slots: vec![] },
            ));
        }
        assert_eq!(
            nested.validated().unwrap_err(),
            "Góc Đông, menu con: Góc Tây: menu con không được lồng nhau"
        );

        let mut unnamed = with_window_submenu();
        if let Action::Submenu { slots } = &mut unnamed.slots[2].action {
            slots[1].label = " ".into();
        }
        assert_eq!(
            unnamed.validated().unwrap_err(),
            "Góc Đông, menu con: Góc Nam: cần tên hiển thị"
        );

        let in_sequence = Action::Sequence {
            steps: vec![Action::Submenu { slots: vec![] }],
        };
        assert!(validate_action(&in_sequence, 0).is_err());
    }

    #[test]
    fn defaults_cover_eight_directions_with_valid_keys() {
        for p in defaults() {
            assert_eq!(p.slots.len(), 8, "{}", p.id);
            for i in 0..8 {
                let s = p
                    .slot(i)
                    .unwrap_or_else(|| panic!("{} thiếu hướng {i}", p.id));
                fn check(a: &Action) {
                    match a {
                        Action::Hotkey { keys } => {
                            keys::parse_chord(keys).unwrap();
                        }
                        Action::Sequence { steps } => steps.iter().for_each(check),
                        _ => {}
                    }
                }
                check(&s.action);
            }
        }
    }

    #[test]
    fn resolve_by_process_case_insensitive() {
        let store = Store::builtin();
        assert_eq!(
            store
                .resolve(Some("chrome.exe"), Some("Chrome_WidgetWin_1"))
                .id,
            "browser"
        );
        assert_eq!(store.resolve(Some("code.exe"), None).id, "code");
        assert_eq!(
            store
                .resolve(Some("EXPLORER.EXE"), Some("CabinetWClass"))
                .id,
            "explorer"
        );
        assert_eq!(
            store.resolve(Some("notepad.exe"), Some("Notepad")).id,
            DEFAULT_ID
        );
        assert_eq!(store.resolve(None, None).id, DEFAULT_ID);
    }

    #[test]
    fn desktop_beats_explorer() {
        let store = Store::builtin();
        assert_eq!(
            store.resolve(Some("explorer.exe"), Some("Progman")).id,
            "desktop"
        );
        assert_eq!(
            store.resolve(Some("explorer.exe"), Some("workerw")).id,
            "desktop"
        );
        // Lớp desktop nhưng process khác thì không phải desktop.
        assert_eq!(
            store.resolve(Some("other.exe"), Some("Progman")).id,
            DEFAULT_ID
        );
        let d = store.profiles.iter().find(|p| p.id == "desktop").unwrap();
        let ne = d.slot(1).unwrap();
        assert_eq!(
            (ne.label.as_str(), ne.action.describe()),
            ("Làm mới", "F5".to_string())
        );
        assert_eq!(d.slot(0).unwrap().label, "Thư mục mới");
    }

    #[test]
    fn disabled_profile_falls_back() {
        let mut store = Store::builtin();
        store
            .profiles
            .iter_mut()
            .find(|p| p.id == "browser")
            .unwrap()
            .enabled = false;
        assert_eq!(store.resolve(Some("chrome.exe"), None).id, DEFAULT_ID);
    }

    #[test]
    fn migrate_adds_only_new_defaults() {
        let dir = std::env::temp_dir().join(format!("radial-migrate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Cấu hình của bản cũ: 4 file, explorer.json đã được sửa tay.
        for p in [browser_profile(), code_profile(), default_profile()] {
            write_profile(&dir, &p).unwrap();
        }
        fs::write(
            dir.join("explorer.json"),
            "{\"id\":\"explorer\",\"name\":\"Của tôi\"}",
        )
        .unwrap();

        let (version, errors) = migrate(&dir, 0);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(version, PROFILES_VERSION);
        assert!(dir.join("desktop.json").exists());
        let mine = fs::read_to_string(dir.join("explorer.json")).unwrap();
        assert!(mine.contains("Của tôi"), "không ghi đè file người dùng");

        // Người dùng xoá desktop.json sau khi đã nâng cấp: không ghi lại.
        fs::remove_file(dir.join("desktop.json")).unwrap();
        migrate(&dir, PROFILES_VERSION);
        assert!(!dir.join("desktop.json").exists());

        // Thư mục trống: ghi cả bộ.
        let empty = dir.join("moi");
        migrate(&empty, 0);
        assert_eq!(fs::read_dir(&empty).unwrap().count(), defaults().len());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_round_trip_matches_plan_format() {
        let p = explorer_profile();
        let text = serde_json::to_string_pretty(&p).unwrap();
        assert!(text.contains("\"match\""));
        assert!(text.contains("\"dir\": \"NE\""));
        assert!(text.contains("\"type\": \"hotkey\""));
        assert!(text.contains("\"clickFirst\": true"));
        assert!(
            text.contains("Thư mục mới"),
            "giữ nguyên tiếng Việt, không escape"
        );
        let back: Profile = serde_json::from_str(&text).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn minimal_json_uses_defaults() {
        let p: Profile = serde_json::from_str(
            r#"{"id":"x","name":"X","slots":[{"dir":"S","label":"Mở","action":{"type":"url","url":"https://example.com/"}}]}"#,
        )
        .unwrap();
        assert!(p.enabled);
        assert!(p.matcher.process.is_empty());
        assert!(p.slot(4).is_some());
        assert!(p.slot(0).is_none());
        assert_eq!(
            p.view(Some("a.exe"), &Context { clipboard: true })
                .slots
                .len(),
            8
        );
        assert_eq!(p.slot(4).unwrap().action.describe(), "example.com");
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("radial-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn validation_rules() {
        let mut p = browser_profile();
        p.matcher.process = vec![" Chrome.EXE ".into(), "chrome.exe".into(), "".into()];
        let ok = p.clone().validated().unwrap();
        assert_eq!(ok.matcher.process, vec!["chrome.exe".to_string()]);

        let mut bad = p.clone();
        bad.id = "Trình duyệt".into();
        assert!(bad.validated().unwrap_err().contains("Mã profile"));

        let mut no_proc = p.clone();
        no_proc.matcher.process.clear();
        assert!(no_proc
            .validated()
            .unwrap_err()
            .contains("tên chương trình"));

        let mut bad_key = p.clone();
        bad_key.slots[0].action = keys_of(&["Ctrl", "Hyper"]);
        let err = bad_key.validated().unwrap_err();
        assert!(
            err.starts_with("Góc Bắc:") && err.contains("Hyper"),
            "{err}"
        );

        let mut dup = p.clone();
        dup.slots[1].dir = DirName::N;
        assert!(dup.validated().unwrap_err().contains("hai góc"));

        let mut empty_label = p;
        empty_label.slots[2].label = "  ".into();
        assert!(empty_label
            .validated()
            .unwrap_err()
            .contains("Góc Đông: cần tên"));

        // Profile Mặc định luôn bật và không khớp theo tên chương trình.
        let mut d = default_profile();
        d.enabled = false;
        d.matcher.process = vec!["x.exe".into()];
        let d = d.validated().unwrap();
        assert!(d.enabled && d.matcher.process.is_empty());
    }

    #[test]
    fn save_rename_and_delete() {
        let dir = temp_dir("save");
        write_defaults(&dir).unwrap();
        let (store, _) = Store::load_dir(&dir);

        // Profile mới.
        let mut notepad = default_profile();
        notepad.id = "notepad".into();
        notepad.name = "Notepad".into();
        notepad.matcher.process = vec!["notepad.exe".into()];
        save_profile(&dir, &store, notepad.clone(), None).unwrap();
        assert!(dir.join("notepad.json").exists());

        // Trùng mã với profile khác.
        let (store, _) = Store::load_dir(&dir);
        let mut clash = notepad.clone();
        clash.id = "browser".into();
        assert!(save_profile(&dir, &store, clash, Some("notepad"))
            .unwrap_err()
            .contains("Đã có"));

        // Đổi mã: file cũ biến mất, file mới xuất hiện.
        let mut renamed = notepad.clone();
        renamed.id = "ghi-chu".into();
        save_profile(&dir, &store, renamed, Some("notepad")).unwrap();
        assert!(!dir.join("notepad.json").exists());
        assert!(dir.join("ghi-chu.json").exists());

        // Xoá.
        let (store, _) = Store::load_dir(&dir);
        delete_profile(&dir, &store, "ghi-chu").unwrap();
        assert!(!dir.join("ghi-chu.json").exists());
        assert!(delete_profile(&dir, &store, DEFAULT_ID).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn paste_needs_clipboard() {
        let explorer = explorer_profile();
        let paste = explorer.slot(DirName::E.index()).unwrap();
        assert!(paste.action.is_paste());
        assert_eq!(paste.condition(), When::Clipboard);
        let empty = Context { clipboard: false };
        let full = Context { clipboard: true };
        assert_eq!(paste.unavailable(&empty), Some("Clipboard trống"));
        assert_eq!(paste.unavailable(&full), None);
        assert!(keys_of(&["shift", "Ins"]).is_paste());
        assert!(!keys_of(&["Ctrl", "Shift", "V"]).is_paste());

        // Góc khác không bị ảnh hưởng; ghi rõ "always" thì Dán luôn bật.
        assert_eq!(
            explorer
                .slot(DirName::N.index())
                .unwrap()
                .unavailable(&empty),
            None
        );
        let mut forced = paste.clone();
        forced.when = Some(When::Always);
        assert_eq!(forced.unavailable(&empty), None);
        let json = serde_json::to_string(&forced).unwrap();
        assert!(json.contains(r#""when":"always""#), "{json}");
        assert!(!serde_json::to_string(paste).unwrap().contains("when"));

        let view = explorer.view(Some("explorer.exe"), &empty);
        assert_eq!(
            view.slots[DirName::E.index()].as_ref().unwrap().unavailable,
            Some("Clipboard trống")
        );
        assert!(view.slots[DirName::N.index()]
            .as_ref()
            .unwrap()
            .unavailable
            .is_none());
    }

    /// Ghi bộ profile mặc định ra JSON (dùng cho bản xem trước giao diện):
    /// `RADIAL_DUMP=out.json cargo test dump_defaults -- --ignored`.
    #[test]
    #[ignore]
    fn dump_defaults() {
        if let Ok(path) = std::env::var("RADIAL_DUMP") {
            fs::write(path, serde_json::to_string_pretty(&defaults()).unwrap()).unwrap();
        }
    }

    #[test]
    fn v3_turns_desktop_terminal_into_settings() {
        let dir = temp_dir("v3");
        fs::create_dir_all(&dir).unwrap();
        // desktop.json kiểu bộ mặc định v2: góc Đông Nam là Mở Terminal.
        let mut old = desktop_profile();
        let se = old.slots.iter_mut().find(|s| s.dir == DirName::SE).unwrap();
        se.label = "Mở Terminal".into();
        se.icon = "terminal".into();
        se.action = Action::Command {
            cmd: "wt".into(),
            args: vec!["-d".into(), "%DESKTOP%".into()],
            cwd: None,
        };
        // Người dùng đã sửa góc khác: phải giữ nguyên.
        old.slots
            .iter_mut()
            .find(|s| s.dir == DirName::N)
            .unwrap()
            .label = "Thư mục".into();
        write_profile(&dir, &old).unwrap();

        let (version, errors) = migrate(&dir, 2);
        assert_eq!((version, errors.len()), (3, 0));
        let (store, _) = Store::load_dir(&dir);
        let desk = store.profiles.iter().find(|p| p.id == "desktop").unwrap();
        let se = desk.slot(DirName::SE.index()).unwrap();
        assert_eq!(
            (se.label.as_str(), &se.action),
            ("Cài đặt Radial Menu", &Action::Settings)
        );
        assert_eq!(desk.slot(DirName::N.index()).unwrap().label, "Thư mục");

        // Góc đã được người dùng đổi thì không đụng tới.
        let mut custom = desk.clone();
        custom
            .slots
            .iter_mut()
            .find(|s| s.dir == DirName::SE)
            .unwrap()
            .label = "Của tôi".into();
        write_profile(&dir, &custom).unwrap();
        migrate(&dir, 2);
        let (store, _) = Store::load_dir(&dir);
        let desk = store.profiles.iter().find(|p| p.id == "desktop").unwrap();
        assert_eq!(desk.slot(DirName::SE.index()).unwrap().label, "Của tôi");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_action_only_at_top_level() {
        let mut p = desktop_profile();
        assert!(p.clone().validated().is_ok());
        p.slots[0].action = Action::Sequence {
            steps: vec![Action::Settings],
        };
        assert!(p.validated().unwrap_err().contains("chuỗi bước"));
        assert_eq!(
            serde_json::to_string(&Action::Settings).unwrap(),
            r#"{"type":"settings"}"#
        );
    }

    #[test]
    fn describe_actions() {
        assert_eq!(keys_of(&["Alt", "Left"]).describe(), "Alt+←");
        let cmd = Action::Command {
            cmd: r"C:\Windows\rundll32.exe".into(),
            args: vec!["a,b".into()],
            cwd: None,
        };
        assert_eq!(cmd.describe(), "rundll32.exe a,b");
        assert_eq!(
            Action::Text {
                text: "Trân trọng,".into()
            }
            .describe(),
            "“Trân trọng,”"
        );
        assert_eq!(
            explorer_profile().slot(3).unwrap().action.describe(),
            "Alt+D → “wt -d .” → Enter"
        );
    }

    #[test]
    fn focus_rules() {
        assert!(keys_of(&["F2"]).needs_focus());
        assert!(!Action::Url {
            url: "https://a.b".into()
        }
        .needs_focus());
        assert!(explorer_profile().slot(3).unwrap().action.needs_focus());
    }

    #[test]
    fn load_dir_writes_defaults_then_reads_them() {
        let dir = std::env::temp_dir().join(format!("radial-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let (store, errors) = Store::load_dir(&dir);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(store.profiles.len(), defaults().len());
        fs::write(dir.join("broken.json"), "{ không phải json").unwrap();
        let (store2, errors2) = Store::load_dir(&dir);
        assert_eq!(store2.profiles.len(), defaults().len());
        assert_eq!(errors2.len(), 1);
        assert!(errors2[0].starts_with("broken.json"));
        let _ = fs::remove_dir_all(&dir);
    }
}
