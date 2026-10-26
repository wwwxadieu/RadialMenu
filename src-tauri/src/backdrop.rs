//! Nền kính mờ cho menu: chụp vùng màn hình dưới đĩa menu, thu nhỏ, làm mờ,
//! tăng bão hoà màu, rồi gửi sang trang menu để vẽ lên canvas.
//!
//! `backdrop-filter` của CSS không dùng được ở đây: trong cửa sổ trong suốt, nó chỉ
//! làm mờ nội dung của chính webview, không làm mờ các app phía sau.
//! Menu chỉ tồn tại khi đang giữ chuột, nên một ảnh tĩnh là đủ.

#![cfg_attr(not(windows), allow(dead_code))]

use serde::Serialize;

/// Cạnh dài nhất của ảnh sau khi thu nhỏ. Ảnh nhỏ nên làm mờ rất nhanh,
/// và đằng nào cũng bị làm mờ nên không mất chi tiết nào đáng kể.
const TARGET: usize = 128;
/// Độ mờ tương đương `blur(20px)` của CSS, tính theo px logic.
const BLUR_SIGMA: f64 = 20.0;
/// Tăng bão hoà như `saturate(170%)`.
const SATURATION: f32 = 1.7;

#[derive(Debug, Clone, Serialize)]
pub struct Backdrop {
    pub seq: u64,
    pub width: u32,
    pub height: u32,
    /// Điểm ảnh RGBA, mã hoá base64.
    pub data: String,
}

/// Ảnh đã làm mờ, RGBA.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Từ ảnh chụp BGRA (thứ tự byte của GDI) ra ảnh nhỏ đã làm mờ, RGBA.
pub fn process(bgra: &[u8], width: usize, height: usize, scale: f64) -> Image {
    assert_eq!(bgra.len(), width * height * 4, "kích thước ảnh không khớp");
    let factor = width.max(height).div_ceil(TARGET).max(1);
    let (sw, sh) = ((width / factor).max(1), (height / factor).max(1));

    // Thu nhỏ bằng cách lấy trung bình từng khối factor × factor.
    let mut channels = vec![vec![0f32; sw * sh]; 3];
    for sy in 0..sh {
        for sx in 0..sw {
            let (mut r, mut g, mut b, mut n) = (0f32, 0f32, 0f32, 0f32);
            for y in sy * factor..((sy + 1) * factor).min(height) {
                for x in sx * factor..((sx + 1) * factor).min(width) {
                    let i = (y * width + x) * 4;
                    b += f32::from(bgra[i]);
                    g += f32::from(bgra[i + 1]);
                    r += f32::from(bgra[i + 2]);
                    n += 1.0;
                }
            }
            let i = sy * sw + sx;
            channels[0][i] = r / n;
            channels[1][i] = g / n;
            channels[2][i] = b / n;
        }
    }

    // Ba lần box blur ≈ Gaussian với sigma ≈ bán kính.
    let radius = ((BLUR_SIGMA * scale.max(0.5)) / factor as f64)
        .round()
        .max(1.0) as usize;
    for channel in &mut channels {
        for _ in 0..3 {
            box_blur(channel, sw, sh, radius);
        }
    }

    let mut rgba = Vec::with_capacity(sw * sh * 4);
    let (reds, greens, blues) = (&channels[0], &channels[1], &channels[2]);
    for ((&r, &g), &b) in reds.iter().zip(greens).zip(blues) {
        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let sat = |c: f32| (luma + (c - luma) * SATURATION).round().clamp(0.0, 255.0) as u8;
        rgba.extend_from_slice(&[sat(r), sat(g), sat(b), 255]);
    }
    Image {
        width: sw,
        height: sh,
        rgba,
    }
}

/// Box blur tách hai chiều, mép ảnh lặp lại điểm ở biên.
fn box_blur(data: &mut [f32], w: usize, h: usize, r: usize) {
    let mut line = Vec::new();
    for y in 0..h {
        line.clear();
        line.extend_from_slice(&data[y * w..(y + 1) * w]);
        blur_line(&line, r, |x, v| data[y * w + x] = v);
    }
    for x in 0..w {
        line.clear();
        line.extend((0..h).map(|y| data[y * w + x]));
        blur_line(&line, r, |y, v| data[y * w + x] = v);
    }
}

fn blur_line(src: &[f32], r: usize, mut write: impl FnMut(usize, f32)) {
    let n = src.len();
    if n == 0 {
        return;
    }
    let at = |i: isize| src[i.clamp(0, n as isize - 1) as usize];
    let window = (2 * r + 1) as f32;
    let mut sum: f32 = (-(r as isize)..=r as isize).map(at).sum();
    for i in 0..n {
        write(i, sum / window);
        let i = i as isize;
        sum += at(i + r as isize + 1) - at(i - r as isize);
    }
}

pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Chụp và làm mờ hình vuông cạnh `size` px vật lý, tâm tại (cx, cy).
#[cfg(windows)]
pub fn capture(seq: u64, cx: i32, cy: i32, size: i32, scale: f64) -> Option<Backdrop> {
    let (bgra, w, h) = win::grab(cx - size / 2, cy - size / 2, size, size)?;
    let image = process(&bgra, w, h, scale);
    Some(Backdrop {
        seq,
        width: image.width as u32,
        height: image.height as u32,
        data: base64(&image.rgba),
    })
}

#[cfg(not(windows))]
pub fn capture(_seq: u64, _cx: i32, _cy: i32, _size: i32, _scale: f64) -> Option<Backdrop> {
    None
}

#[cfg(windows)]
mod win;

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize, bgr: [u8; 3]) -> Vec<u8> {
        (0..w * h)
            .flat_map(|_| [bgr[0], bgr[1], bgr[2], 255])
            .collect()
    }

    #[test]
    fn downscales_to_target_size() {
        let img = process(&solid(516, 516, [0, 0, 0]), 516, 516, 1.5);
        assert_eq!((img.width, img.height), (103, 103));
        assert_eq!(img.rgba.len(), 103 * 103 * 4);
        let small = process(&solid(64, 32, [0, 0, 0]), 64, 32, 1.0);
        assert_eq!((small.width, small.height), (64, 32));
    }

    #[test]
    fn solid_color_stays_the_same_and_turns_rgba() {
        // BGR (30, 60, 200) → RGB (200, 60, 30), bão hoà tăng nhưng vẫn trong 0..255.
        let img = process(&solid(200, 200, [30, 60, 200]), 200, 200, 1.0);
        let px = &img.rgba[0..4];
        assert_eq!(px[3], 255);
        assert!(
            px[0] > px[1] && px[1] > px[2],
            "giữ đúng thứ tự kênh màu: {px:?}"
        );
        assert!(
            img.rgba.chunks(4).all(|p| p == px),
            "màu đồng nhất thì sau khi làm mờ vẫn đồng nhất"
        );
        let gray = process(&solid(50, 50, [128, 128, 128]), 50, 50, 1.0);
        assert_eq!(
            &gray.rgba[0..4],
            &[128, 128, 128, 255],
            "màu xám không bị đổi khi tăng bão hoà"
        );
    }

    #[test]
    fn blur_spreads_a_hard_edge() {
        // Nửa trái đen, nửa phải trắng: sau khi làm mờ, giữa ảnh phải là màu xám.
        let (w, h) = (128, 16);
        let mut bgra = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in w / 2..w {
                let i = (y * w + x) * 4;
                bgra[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let img = process(&bgra, w, h, 1.0);
        let mid = img.rgba[(8 * img.width + img.width / 2) * 4];
        assert!(
            (60..=200).contains(&mid),
            "giữa mép phải chuyển dần, được {mid}"
        );
        assert!(img.rgba[(8 * img.width) * 4] < 40);
        assert!(img.rgba[(8 * img.width + img.width - 1) * 4] > 215);
    }

    #[test]
    fn base64_matches_standard() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b""), "");
        assert_eq!(base64(&[0xFF, 0xEE, 0xDD, 0xCC]), "/+7dzA==");
    }
}
