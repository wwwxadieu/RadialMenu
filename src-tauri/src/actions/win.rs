//! Gửi phím, gõ chữ, click, kích hoạt cửa sổ và mở URL trên Windows.

use core::ffi::c_void;
use std::time::Duration;

use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, VkKeyScanW, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MAPVK_VK_TO_VSC,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, SHGetKnownFolderPath, ShellExecuteW,
    KF_FLAG_DEFAULT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowThreadProcessId, SetCursorPos,
    SetForegroundWindow, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
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
