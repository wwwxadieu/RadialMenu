//! Đọc/ghi khoá Run của người dùng hiện tại.

use core::ffi::c_void;

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

pub fn write(name: &str, command: &str) -> Result<(), String> {
    let data: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            RUN,
            &HSTRING::from(name),
            REG_SZ.0,
            Some(data.as_ptr() as *const c_void),
            (data.len() * 2) as u32,
        )
    };
    result
        .ok()
        .map_err(|e| format!("Không bật được chạy cùng Windows: {}", e.message()))
}

pub fn remove(name: &str) -> Result<(), String> {
    let result = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN, &HSTRING::from(name)) };
    if result == ERROR_SUCCESS || result == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(format!(
            "Không tắt được chạy cùng Windows: {}",
            result.to_hresult().message()
        ))
    }
}

pub fn read(name: &str) -> Option<String> {
    let mut buf = [0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN,
            &HSTRING::from(name),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut c_void),
            Some(&mut size),
        )
    };
    if result != ERROR_SUCCESS {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}
