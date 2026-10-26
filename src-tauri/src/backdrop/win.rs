//! Chụp một vùng màn hình bằng GDI. Cửa sổ menu được đánh dấu
//! WDA_EXCLUDEFROMCAPTURE nên không lọt vào ảnh chụp.

use core::ffi::c_void;

use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};

/// Trả về (điểm ảnh BGRA từ trên xuống, rộng, cao).
pub fn grab(x: i32, y: i32, w: i32, h: i32) -> Option<(Vec<u8>, usize, usize)> {
    if w <= 0 || h <= 0 {
        return None;
    }
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        let previous = SelectObject(memory, bitmap.into());

        let copied = BitBlt(memory, 0, 0, w, h, Some(screen), x, y, SRCCOPY).is_ok();

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // số âm = ảnh từ trên xuống
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let lines = if copied {
            GetDIBits(
                memory,
                bitmap,
                0,
                h as u32,
                Some(pixels.as_mut_ptr() as *mut c_void),
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };

        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);

        (lines == h).then_some((pixels, w as usize, h as usize))
    }
}
