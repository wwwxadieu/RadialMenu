use core::ffi::c_void;
use std::sync::OnceLock;

use windows::core::BOOL;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, POINT, TRUE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken,
    QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    WindowFromPoint, GA_ROOT, GWL_EXSTYLE, GW_OWNER, WS_EX_TOOLWINDOW,
};

use super::{AppInfo, Target};

pub fn running_apps() -> Vec<AppInfo> {
    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let list = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        list.push(hwnd);
        TRUE
    }
    let mut handles: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(collect),
            LPARAM(&mut handles as *mut Vec<HWND> as isize),
        );
    }

    let own = unsafe { GetCurrentProcessId() };
    let mut apps: Vec<AppInfo> = Vec::new();
    for hwnd in handles {
        unsafe {
            // Chỉ cửa sổ chính đang hiện: không phải hộp thoại con, không phải tool window.
            if !IsWindowVisible(hwnd).as_bool()
                || GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid())
            {
                continue;
            }
            if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
                continue;
            }
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                continue;
            }
            let mut buf = vec![0u16; len as usize + 1];
            let n = GetWindowTextW(hwnd, &mut buf);
            let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 || pid == own {
                continue;
            }
            let process = process_info(pid).name;
            if process.is_empty() || apps.iter().any(|a| a.process == process) {
                continue;
            }
            apps.push(AppInfo { process, title });
        }
    }
    apps
}

pub fn at_point(x: i32, y: i32) -> Option<Target> {
    unsafe {
        let hit = WindowFromPoint(POINT { x, y });
        if hit.is_invalid() {
            return None;
        }
        let root = GetAncestor(hit, GA_ROOT);
        let root = if root.is_invalid() { hit } else { root };

        let mut pid = 0u32;
        GetWindowThreadProcessId(root, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let info = process_info(pid);
        Some(Target {
            hwnd: root.0 as isize,
            pid,
            process: info.name,
            path: info.path,
            class: class_name(root),
            elevated: info.elevated && !self_elevated(),
            fullscreen: is_fullscreen(root),
        })
    }
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

struct ProcessInfo {
    /// Đường dẫn đầy đủ của file exe.
    path: String,
    /// Tên file, chữ thường.
    name: String,
    elevated: bool,
}

fn process_info(pid: u32) -> ProcessInfo {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return ProcessInfo {
                path: String::new(),
                name: String::new(),
                elevated: false,
            };
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let path = match QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        ) {
            Ok(()) => String::from_utf16_lossy(&buf[..len as usize]),
            Err(_) => String::new(),
        };
        let name = path.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
        // Không mở được token của app khác thường là vì app đó chạy quyền cao hơn.
        let elevated = token_elevated(handle).unwrap_or(true);
        let _ = CloseHandle(handle);
        ProcessInfo {
            path,
            name,
            elevated,
        }
    }
}

fn token_elevated(process: HANDLE) -> Option<bool> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut info = TOKEN_ELEVATION::default();
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut info as *mut TOKEN_ELEVATION as *mut c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok.then_some(info.TokenIsElevated != 0)
    }
}

fn self_elevated() -> bool {
    static SELF: OnceLock<bool> = OnceLock::new();
    *SELF.get_or_init(|| token_elevated(unsafe { GetCurrentProcess() }).unwrap_or(false))
}

/// Chỉ tính là toàn màn hình khi chính cửa sổ này đang active và Windows báo có
/// app toàn màn hình (cùng cách Windows quyết định tắt thông báo khi chơi game).
fn is_fullscreen(root: HWND) -> bool {
    unsafe {
        if GetForegroundWindow() != root {
            return false;
        }
        matches!(
            SHQueryUserNotificationState(),
            Ok(state) if state == QUNS_BUSY || state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE
        )
    }
}
