//! Gửi phím, gõ chữ, click, kích hoạt cửa sổ và mở URL trên Windows.

use core::ffi::c_void;
use std::time::Duration;

use windows::core::{Interface, GUID, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, IServiceProvider, CLSCTX_LOCAL_SERVER,
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, VkKeyScanW, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, IFolderView, IShellBrowser,
    IShellItemArray, IShellWindows, IWebBrowserApp, SHGetKnownFolderPath, SHOpenWithDialog,
    SID_STopLevelBrowser, ShellExecuteW, ShellWindows, CSIDL_DESKTOP, KF_FLAG_DEFAULT,
    OAIF_ALLOW_REGISTRATION, OAIF_EXEC, OPENASINFO, SIGDN_FILESYSPATH, SVGIO_SELECTION,
    SWC_DESKTOP, SWFO_NEEDDISPATCH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowThreadProcessId,
    SetCursorPos, SetForegroundWindow, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, SW_SHOWNORMAL,
};

use crate::hook::SIGNATURE;
use crate::keys::{self, Key};

pub fn init_thread() {
    // ShellExecute cần COM trên thread gọi nó.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    }
}

fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// Đưa cửa sổ dưới con trỏ lên làm cửa sổ active, giống như chuột phải gốc vẫn làm.
pub fn activate(raw: isize) {
    let target = HWND(raw as *mut c_void);
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground == target {
            return;
        }
        // Windows chỉ cho đổi cửa sổ active khi luồng gọi "dính" với luồng đang active.
        let fg_thread = if foreground.is_invalid() {
            0
        } else {
            GetWindowThreadProcessId(foreground, None)
        };
        let me = GetCurrentThreadId();
        let attached =
            fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        let _ = SetForegroundWindow(target);
        if attached {
            let _ = AttachThreadInput(me, fg_thread, false);
        }
    }
    sleep_ms(40);
}

fn mouse_input(dx: i32, dy: i32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    }
}

/// Click trái tại (x, y) rồi đưa con trỏ về chỗ cũ.
pub fn click_at(x: i32, y: i32) {
    unsafe {
        let mut back = POINT::default();
        let _ = GetCursorPos(&mut back);

        // Toạ độ tuyệt đối trên toàn bộ các màn hình, chuẩn hoá về 0..65535.
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(2);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(2);
        let nx = (i64::from(x - vx) * 65535 / i64::from(vw - 1)) as i32;
        let ny = (i64::from(y - vy) * 65535 / i64::from(vh - 1)) as i32;

        let inputs = [
            mouse_input(
                nx,
                ny,
                MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
            ),
            mouse_input(0, 0, MOUSEEVENTF_LEFTDOWN),
            mouse_input(0, 0, MOUSEEVENTF_LEFTUP),
        ];
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        sleep_ms(30);
        let _ = SetCursorPos(back.x, back.y);
    }
    sleep_ms(40);
}

fn key_input(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    }
}

fn press(inputs: &mut Vec<INPUT>, vk: u16, extended: bool, up: bool) {
    let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) } as u16;
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    inputs.push(key_input(vk, scan, flags));
}

/// Phím ký tự: tra lại theo bố cục bàn phím đang dùng (US, Pháp…); (vk, cần Shift).
fn resolve(key: &Key) -> (u16, bool) {
    if let Some(ch) = key.ch {
        let mut buf = [0u16; 2];
        if ch.encode_utf16(&mut buf).len() == 1 {
            let r = unsafe { VkKeyScanW(buf[0]) };
            if r != -1 {
                return ((r as u16) & 0xFF, (r as u16) & 0x0100 != 0);
            }
        }
    }
    (key.vk, key.shift)
}

fn send(inputs: &[INPUT]) -> Result<(), String> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err("Windows không nhận phím giả lập (app đích có thể chạy quyền Admin)".into())
    }
}

/// Nhấn các phím theo thứ tự rồi nhả theo thứ tự ngược lại, trong một lần SendInput.
pub fn send_keys(chord: &[Key]) -> Result<(), String> {
    let has_shift = chord.iter().any(|k| k.vk == keys::VK_SHIFT);
    let mut downs: Vec<(u16, bool)> = Vec::new();
    for key in chord {
        let (vk, needs_shift) = resolve(key);
        if needs_shift && !has_shift && !downs.iter().any(|&(v, _)| v == keys::VK_SHIFT) {
            downs.push((keys::VK_SHIFT, false));
        }
        downs.push((vk, key.extended));
    }
    let mut inputs = Vec::with_capacity(downs.len() * 2);
    for &(vk, ext) in &downs {
        press(&mut inputs, vk, ext, false);
    }
    for &(vk, ext) in downs.iter().rev() {
        press(&mut inputs, vk, ext, true);
    }
    send(&inputs)
}

/// Gõ chữ Unicode; xuống dòng và tab gửi bằng phím thật để app hiểu đúng.
pub fn type_text(text: &str) -> Result<(), String> {
    let mut inputs = Vec::new();
    for ch in text.chars() {
        match ch {
            '\r' => {}
            '\n' => {
                press(&mut inputs, keys::VK_RETURN, false, false);
                press(&mut inputs, keys::VK_RETURN, false, true);
            }
            '\t' => {
                press(&mut inputs, keys::VK_TAB, false, false);
                press(&mut inputs, keys::VK_TAB, false, true);
            }
            _ => {
                let mut buf = [0u16; 2];
                for &unit in ch.encode_utf16(&mut buf).iter() {
                    inputs.push(key_input(0, unit, KEYEVENTF_UNICODE));
                    inputs.push(key_input(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
                }
            }
        }
    }
    for chunk in inputs.chunks(64) {
        send(chunk)?;
    }
    Ok(())
}

/// Đường dẫn thật của thư mục Desktop/Documents/Downloads (kể cả khi nằm trong OneDrive).
pub fn known_folder(name: &str) -> Option<String> {
    let id: &GUID = match name.to_ascii_uppercase().as_str() {
        "DESKTOP" => &FOLDERID_Desktop,
        "DOCUMENTS" => &FOLDERID_Documents,
        "DOWNLOADS" => &FOLDERID_Downloads,
        _ => return None,
    };
    unsafe {
        let path = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let text = path.to_string().ok();
        CoTaskMemFree(Some(path.0 as *const c_void));
        text
    }
}

/// Mở URL, file hoặc thư mục bằng app mặc định của Windows.
pub fn open(target: &str) -> Result<(), String> {
    let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(wide.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute trả về giá trị > 32 khi thành công.
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("không mở được {target}"))
    }
}

// ---------- Mở bằng: file đang chọn trong File Explorer / Desktop ----------

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// VARIANT kiểu số nguyên 32 bit (tham số của IShellWindows).
fn variant_i4(value: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: core::mem::ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_I4,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { lVal: value },
            }),
        },
    }
}

fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

/// Đường dẫn các mục đang chọn trong một cửa sổ Explorer (qua dịch vụ của trình duyệt shell).
unsafe fn selection_of(provider: &IServiceProvider) -> windows::core::Result<Vec<String>> {
    unsafe {
        let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
        let view = browser.QueryActiveShellView()?;
        let folder: IFolderView = view.cast()?;
        // Không có mục nào được chọn thì Explorer trả lỗi: coi như danh sách rỗng.
        let Ok(items) = folder.Items::<IShellItemArray>(SVGIO_SELECTION) else {
            return Ok(Vec::new());
        };
        let mut paths = Vec::new();
        for i in 0..items.GetCount()? {
            let Ok(item) = items.GetItemAt(i) else {
                continue;
            };
            // Mục không phải file thật (This PC, thùng rác…) không có đường dẫn: bỏ qua.
            let Ok(name) = item.GetDisplayName(SIGDN_FILESYSPATH) else {
                continue;
            };
            if let Ok(path) = name.to_string() {
                paths.push(path);
            }
            CoTaskMemFree(Some(name.0 as *const c_void));
        }
        Ok(paths)
    }
}

/// File đang chọn trong cửa sổ File Explorer `target` (HWND gốc dưới điểm nhấn),
/// hoặc trên Desktop nếu `target` là màn hình nền.
pub fn selected_files(target: Option<isize>) -> Result<Vec<String>, String> {
    const NOT_EXPLORER: &str =
        "Mở bằng chỉ dùng được với file trong File Explorer hoặc trên Desktop";
    let root = target.map(|t| HWND(t as *mut c_void)).ok_or(NOT_EXPLORER)?;
    let class = class_of(root);
    let fail = |e: windows::core::Error| format!("không đọc được file đang chọn: {e}");
    unsafe {
        let windows: IShellWindows =
            CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER).map_err(fail)?;
        if class == "Progman" || class == "WorkerW" {
            let mut hwnd = 0i32;
            let desktop = windows
                .FindWindowSW(
                    &variant_i4(CSIDL_DESKTOP as i32),
                    &VARIANT::default(),
                    SWC_DESKTOP,
                    &mut hwnd,
                    SWFO_NEEDDISPATCH,
                )
                .map_err(fail)?;
            return selection_of(&desktop.cast::<IServiceProvider>().map_err(fail)?).map_err(fail);
        }
        for i in 0..windows.Count().map_err(fail)? {
            let Ok(item) = windows.Item(&variant_i4(i)) else {
                continue;
            };
            let Ok(browser) = item.cast::<IWebBrowserApp>() else {
                continue;
            };
            match browser.HWND() {
                Ok(h) if h.0 == root.0 as isize => {}
                _ => continue,
            }
            return selection_of(&browser.cast::<IServiceProvider>().map_err(fail)?).map_err(fail);
        }
    }
    Err(NOT_EXPLORER.into())
}

/// Chạy `app` với các file làm tham số (mỗi file một tham số, có ngoặc kép).
pub fn launch_with(app: &str, files: &[String]) -> Result<(), String> {
    let params = files
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let (verb, file, args) = (wide("open"), wide(app), wide(&params));
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(args.as_ptr()),
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!(
            "không chạy được {app}: kiểm tra tên hoặc đường dẫn chương trình"
        ))
    }
}

/// Hộp thoại "Open with" của Windows cho `file`. Hộp thoại chờ người dùng chọn, nên chạy
/// trên thread riêng để các lệnh sau không phải đợi.
pub fn open_with_dialog(owner: Option<isize>, file: &str) -> Result<(), String> {
    let file = file.to_string();
    std::thread::Builder::new()
        .name("radial-open-with".into())
        .spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
            let path = wide(&file);
            let info = OPENASINFO {
                pcszFile: PCWSTR(path.as_ptr()),
                pcszClass: PCWSTR::null(),
                oaifInFlags: OAIF_ALLOW_REGISTRATION | OAIF_EXEC,
            };
            // Gắn vào cửa sổ Explorer để hộp thoại hiện ngay trên nó.
            let parent = owner.map(|o| HWND(o as *mut c_void));
            let _ = SHOpenWithDialog(parent, &info);
        })
        .map(|_| ())
        .map_err(|e| format!("không mở được hộp thoại chọn app: {e}"))
}
