//! Đọc icon từ file exe (SHDefExtractIcon) hoặc từ cửa sổ (WM_GETICON), rồi lấy điểm ảnh.

use core::ffi::c_void;

use windows::core::HSTRING;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::UI::Shell::SHDefExtractIconW;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetClassLongPtrW, GetIconInfo, SendMessageTimeoutW, GCLP_HICON, HICON, ICONINFO,
    ICON_BIG, SMTO_ABORTIFHUNG, WM_GETICON,
};

/// (BGRA, mask BGRA nếu có, rộng, cao).
pub type Pixels = (Vec<u8>, Option<Vec<u8>>, u32, u32);

pub fn load(path: &str, hwnd: isize, size: u32) -> Option<Pixels> {
    unsafe {
        if !path.is_empty() {
            let mut large = HICON::default();
            // Từ thấp = cỡ icon lớn, từ cao = cỡ icon nhỏ (0 = không cần).
            let found = SHDefExtractIconW(
                &HSTRING::from(path),
                0,
                0,
                Some(&mut large),
                None,
                size & 0xFFFF,
            )
            .is_ok();
            if found && !large.is_invalid() {
                let px = pixels(large);
                let _ = DestroyIcon(large);
                if px.is_some() {
                    return px;
                }
            }
        }

        // File exe không có icon: hỏi cửa sổ. Icon này thuộc cửa sổ nên không huỷ.
        let window = HWND(hwnd as *mut c_void);
        let mut result = 0usize;
        let sent = SendMessageTimeoutW(
            window,
            WM_GETICON,
            WPARAM(ICON_BIG as usize),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            50,
            Some(&mut result),
        );
        let mut icon = HICON(result as *mut c_void);
        if sent.0 == 0 || icon.is_invalid() {
            icon = HICON(GetClassLongPtrW(window, GCLP_HICON) as *mut c_void);
        }
        if icon.is_invalid() {
            return None;
        }
        pixels(icon)
    }
}

unsafe fn pixels(icon: HICON) -> Option<Pixels> {
    let mut info = ICONINFO::default();
    unsafe { GetIconInfo(icon, &mut info) }.ok()?;
    let (color, mask) = (info.hbmColor, info.hbmMask);

    let result = (|| {
        if color.is_invalid() {
            return None; // icon đen trắng: bỏ qua, dùng icon profile
        }
        let mut bm = BITMAP::default();
        let read = unsafe {
            GetObjectW(
                color.into(),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bm as *mut BITMAP as *mut c_void),
            )
        };
        if read == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
            return None;
        }
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        let bgra = unsafe { dib(color, w, h) }?;
        let mask_px = if mask.is_invalid() {
            None
        } else {
            unsafe { dib(mask, w, h) }
        };
        Some((bgra, mask_px, w as u32, h as u32))
    })();

    unsafe {
        if !color.is_invalid() {
            let _ = DeleteObject(color.into());
        }
        if !mask.is_invalid() {
            let _ = DeleteObject(mask.into());
        }
    }
    result
}

/// Điểm ảnh 32 bit từ trên xuống của một bitmap.
unsafe fn dib(bitmap: HBITMAP, w: i32, h: i32) -> Option<Vec<u8>> {
    unsafe {
        let dc = GetDC(None);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let lines = GetDIBits(
            dc,
            bitmap,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut c_void),
            &mut info,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, dc);
        (lines == h).then_some(buf)
    }
}
