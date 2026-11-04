//! Bộ nhận cử chỉ nhấn giữ chuột phải → kéo → nhả.
//!
//! Đây là một máy trạng thái thuần, không gọi API Windows nào, để test được
//! trên mọi nền tảng. `hook` đưa sự kiện chuột vào đây và làm theo kết quả:
//! nuốt hay cho qua sự kiện, và phát hiệu ứng nào.

// Ngoài Windows chưa có hook nào dùng tới máy trạng thái này (chỉ có test).
#![cfg_attr(not(windows), allow(dead_code))]

/// Hướng trong menu: 0 = Bắc, tăng theo chiều kim đồng hồ (1 = Đông Bắc … 7 = Tây Bắc).
pub type Dir = u8;

/// Tên tiếng Việt của 8 hướng, theo đúng thứ tự `Dir`.
pub const DIR_NAMES: [&str; 8] = [
    "Bắc",
    "Đông Bắc",
    "Đông",
    "Đông Nam",
    "Nam",
    "Tây Nam",
    "Tây",
    "Tây Bắc",
];

/// Trong khoảng này ngay sau khi menu mở, kéo xa không làm menu tự đóng: cú vụt
/// để mở và chọn nhanh (nhấn, vụt, nhả) thường đi rất xa trong tích tắc.
pub const FAR_GRACE_MS: u32 = 250;

/// Menu con mở bằng cách nhả chuột (chế độ bấm chọn) tự huỷ nếu bỏ không quá lâu.
pub const STICKY_TIMEOUT_MS: u32 = 15_000;

/// Nút chuột khác nút phải, để nuốt đúng lần nhả của cú bấm chọn trong menu con.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    X,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// Khoảng kéo tối thiểu để mở menu, tính bằng px logic (màn hình scale 100%).
    pub threshold_px: f64,
    /// Bán kính vùng huỷ ở giữa menu, px logic. Nhả trong vùng này là huỷ.
    pub cancel_radius_px: f64,
    /// Kéo ra xa quá khoảng này (px logic, tính từ điểm nhấn) thì menu tự đóng.
    /// `None` = không bao giờ tự đóng.
    pub close_distance_px: Option<f64>,
    /// Kéo qua mép đĩa menu ở một góc có menu con thì mở menu con (px logic, đã nhân
    /// kích thước menu). Bấm ngoài vòng này khi đang ở chế độ bấm chọn là huỷ.
    pub drill_radius_px: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            threshold_px: 10.0,
            cancel_radius_px: 30.0,
            close_distance_px: Some(450.0),
            drill_radius_px: 165.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Không có gì đang diễn ra.
    Idle,
    /// Đang giữ chuột phải nhưng chưa kéo quá ngưỡng. Cú nhấn đã bị giữ lại.
    Pressed,
    /// Menu đang mở, người dùng đang chọn hướng (vẫn giữ chuột phải).
    Open,
    /// Menu con đã mở bằng cách nhả chuột ở góc của nó: di chuột rồi bấm để chọn.
    Sticky,
    /// Đã trả nút phải cho Windows; lần nhả tới sẽ đi thẳng.
    Passthrough,
    /// Menu vừa bị huỷ (Esc, nút khác, kéo quá xa); nuốt lần nhả chuột phải còn lại.
    Cancelled,
}

/// Hook nên làm gì với sự kiện vừa nhận.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Swallow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    None,
    /// Menu mở, tâm đặt tại điểm nhấn (toạ độ màn hình vật lý).
    Open {
        x: i32,
        y: i32,
        scale: f64,
        dir: Option<Dir>,
    },
    /// Hướng đang chỉ thay đổi. `None` = đang ở vùng huỷ giữa menu.
    Dir(Option<Dir>),
    /// Menu đóng. `Some` = chạy lệnh của hướng đó, `None` = huỷ.
    Release(Option<Dir>),
    /// Kéo ra quá xa: menu tự đóng, không chạy gì.
    Away,
    /// Mở menu con của góc `dir` ngay tại chỗ. `sticky` = mở bằng cách nhả chuột,
    /// giờ di chuột rồi bấm để chọn; ngược lại vẫn đang giữ chuột phải.
    Drill {
        dir: Dir,
        sticky: bool,
    },
    /// Nhả trước ngưỡng: phát lại một cú right-click đầy đủ tại điểm nhấn.
    ReplayClick {
        x: i32,
        y: i32,
    },
    /// Có nút khác được nhấn khi đang giữ: trả lại cú nhấn phải cho Windows,
    /// lần nhả để Windows tự xử lý.
    ReplayDown {
        x: i32,
        y: i32,
    },
}

#[derive(Debug)]
pub struct Gesture {
    cfg: Config,
    phase: Phase,
    origin: (i32, i32),
    scale: f64,
    dir: Option<Dir>,
    /// Thời điểm menu mở (ms, đồng hồ của sự kiện chuột).
    opened_at: u32,
    /// Các góc của menu đang mở có menu con (bit `1 << dir`). App báo sau khi biết profile.
    submenus: u8,
    /// Đang ở menu con.
    in_submenu: bool,
    /// Lần nhả của nút vừa dùng để bấm chọn trong menu con: nuốt luôn.
    swallow_up: Option<Button>,
    /// Thời điểm sự kiện chuột gần nhất (ms).
    last_time: u32,
}

impl Gesture {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            phase: Phase::Idle,
            origin: (0, 0),
            scale: 1.0,
            dir: None,
            opened_at: 0,
            submenus: 0,
            in_submenu: false,
            swallow_up: None,
            last_time: 0,
        }
    }

    #[cfg(test)]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn set_config(&mut self, cfg: Config) {
        self.cfg = cfg;
    }

    /// Góc nào của menu đang mở có menu con (bit `1 << dir`).
    pub fn set_submenus(&mut self, mask: u8) {
        self.submenus = mask;
    }

    fn has_submenu(&self, dir: Option<Dir>) -> bool {
        !self.in_submenu && dir.is_some_and(|d| self.submenus & (1 << d) != 0)
    }

    /// Chuột phải được nhấn. `scale` = DPI của màn hình chứa con trỏ / 96.
    /// `bypass` = đang tạm dừng hoặc giữ Shift: để Windows xử lý như thường.
    pub fn right_down(&mut self, x: i32, y: i32, scale: f64, bypass: bool) -> (Verdict, Effect) {
        // Menu con đang chờ bấm chọn: cú nhấn phải là cú bấm chọn.
        if self.phase == Phase::Sticky {
            if let Some(result) = self.sticky_click(x, y, None) {
                return result;
            }
        }
        // Một cú nhấn mới luôn bắt đầu lại từ đầu, kể cả khi trạng thái cũ còn kẹt
        // (ví dụ lỡ mất sự kiện nhả lúc Windows chuyển sang màn hình UAC).
        self.dir = None;
        self.in_submenu = false;
        self.submenus = 0;
        self.swallow_up = None;
        if bypass {
            self.phase = Phase::Passthrough;
            return (Verdict::Pass, Effect::None);
        }
        self.phase = Phase::Pressed;
        self.origin = (x, y);
        self.scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        (Verdict::Swallow, Effect::None)
    }

    /// Chuột di chuyển. Sự kiện di chuyển luôn được cho qua để con trỏ vẫn chạy.
    /// `time` = thời điểm của sự kiện (ms, `MSLLHOOKSTRUCT::time`).
    pub fn mouse_move(&mut self, x: i32, y: i32, time: u32) -> Effect {
        match self.phase {
            Phase::Pressed => {
                let (dx, dy) = self.offset(x, y);
                if dx.hypot(dy) < self.cfg.threshold_px * self.scale {
                    return Effect::None;
                }
                self.phase = Phase::Open;
                self.opened_at = time;
                self.dir = direction(dx, dy, self.cfg.cancel_radius_px * self.scale);
                Effect::Open {
                    x: self.origin.0,
                    y: self.origin.1,
                    scale: self.scale,
                    dir: self.dir,
                }
            }
            Phase::Open | Phase::Sticky => {
                self.last_time = time;
                let (dx, dy) = self.offset(x, y);
                let dist = dx.hypot(dy);
                let settled = time.wrapping_sub(self.opened_at) >= FAR_GRACE_MS;
                if let (Some(limit), true) = (self.cfg.close_distance_px, settled) {
                    if dist > limit * self.scale {
                        // Đang giữ chuột phải thì còn một lần nhả phải nuốt.
                        self.phase = if self.phase == Phase::Open {
                            Phase::Cancelled
                        } else {
                            Phase::Idle
                        };
                        self.dir = None;
                        return Effect::Away;
                    }
                }
                let dir = direction(dx, dy, self.cfg.cancel_radius_px * self.scale);
                // Kéo qua mép đĩa ở góc có menu con: mở menu con ngay tại chỗ, giữ
                // nguyên hướng đang chỉ (góc cùng hướng của menu con sáng lên).
                if self.phase == Phase::Open
                    && self.has_submenu(dir)
                    && dist > self.cfg.drill_radius_px * self.scale
                {
                    self.in_submenu = true;
                    self.opened_at = time;
                    self.dir = dir;
                    return Effect::Drill {
                        dir: dir.unwrap_or_default(),
                        sticky: false,
                    };
                }
                if dir == self.dir {
                    return Effect::None;
                }
                self.dir = dir;
                Effect::Dir(dir)
            }
            _ => Effect::None,
        }
    }

    /// Chuột phải được nhả.
    pub fn right_up(&mut self) -> (Verdict, Effect) {
        let phase = std::mem::replace(&mut self.phase, Phase::Idle);
        match phase {
            // Cú nhấn xảy ra trước khi app chạy: không phải việc của mình.
            Phase::Idle | Phase::Passthrough => (Verdict::Pass, Effect::None),
            Phase::Pressed => {
                let (x, y) = self.origin;
                (Verdict::Swallow, Effect::ReplayClick { x, y })
            }
            // Nhả ở góc có menu con: mở menu con và chờ bấm chọn.
            Phase::Open if self.has_submenu(self.dir) => {
                self.phase = Phase::Sticky;
                self.in_submenu = true;
                self.opened_at = self.last_time;
                (
                    Verdict::Swallow,
                    Effect::Drill {
                        dir: self.dir.unwrap_or_default(),
                        sticky: true,
                    },
                )
            }
            Phase::Open => (Verdict::Swallow, Effect::Release(self.dir.take())),
            Phase::Cancelled => (Verdict::Swallow, Effect::None),
            // Nhả phải mà không có cú nhấn tương ứng: menu con vẫn chờ bấm chọn.
            Phase::Sticky => {
                self.phase = Phase::Sticky;
                (Verdict::Pass, Effect::None)
            }
        }
    }

    /// Hook gọi trước mỗi sự kiện chuột với thời điểm của nó. Menu con đang chờ bấm
    /// chọn mà bị bỏ không quá lâu thì tự huỷ (trả `Release(None)` để đóng menu), rồi
    /// sự kiện đó được xử lý như bình thường.
    pub fn note_time(&mut self, time: u32) -> Effect {
        self.last_time = time;
        if self.phase == Phase::Sticky && time.wrapping_sub(self.opened_at) > STICKY_TIMEOUT_MS {
            self.reset();
            return Effect::Release(None);
        }
        Effect::None
    }

    /// Bấm (nút bất kỳ) khi menu con đang chờ bấm chọn. `None` = không phải việc của
    /// menu con (đã hết hạn): xử lý như bình thường.
    fn sticky_click(
        &mut self,
        x: i32,
        y: i32,
        button: Option<Button>,
    ) -> Option<(Verdict, Effect)> {
        let (dx, dy) = self.offset(x, y);
        let dir = self.dir;
        self.reset();
        if dx.hypot(dy) > self.cfg.drill_radius_px * self.scale * 1.2 {
            // Bấm hẳn ra ngoài menu: đóng menu, cú bấm đi tiếp tới Windows như thường.
            return Some((Verdict::Pass, Effect::Release(None)));
        }
        match button {
            // Nút phải: nuốt cả lần nhả sắp tới.
            None => self.phase = Phase::Cancelled,
            Some(b) => self.swallow_up = Some(b),
        }
        Some((Verdict::Swallow, Effect::Release(dir)))
    }

    fn reset(&mut self) {
        self.phase = Phase::Idle;
        self.dir = None;
        self.in_submenu = false;
        self.submenus = 0;
    }

    /// Nút trái, giữa hoặc nút phụ được nhấn tại (x, y). Thường được cho qua;
    /// riêng khi menu con đang chờ bấm chọn thì đó là cú bấm chọn và bị nuốt.
    pub fn other_button_down(&mut self, button: Button, x: i32, y: i32) -> (Verdict, Effect) {
        match self.phase {
            Phase::Pressed => {
                // Có thể là thao tác bấm hai nút cùng lúc: trả nút phải về cho Windows.
                self.phase = Phase::Passthrough;
                let (x, y) = self.origin;
                (Verdict::Pass, Effect::ReplayDown { x, y })
            }
            Phase::Open => {
                self.phase = Phase::Cancelled;
                self.dir = None;
                (Verdict::Pass, Effect::Release(None))
            }
            Phase::Sticky => self
                .sticky_click(x, y, Some(button))
                .unwrap_or((Verdict::Pass, Effect::None)),
            _ => (Verdict::Pass, Effect::None),
        }
    }

    /// Nút trái, giữa hoặc nút phụ được nhả: nuốt nếu đó là lần nhả của cú bấm chọn.
    pub fn other_button_up(&mut self, button: Button) -> Verdict {
        if self.swallow_up == Some(button) {
            self.swallow_up = None;
            Verdict::Swallow
        } else {
            Verdict::Pass
        }
    }

    /// Phím Esc được nhấn. Chỉ có tác dụng khi menu đang mở.
    pub fn escape(&mut self) -> (Verdict, Effect) {
        match self.phase {
            Phase::Open => {
                self.phase = Phase::Cancelled;
                self.dir = None;
                (Verdict::Swallow, Effect::Release(None))
            }
            Phase::Sticky => {
                self.reset();
                (Verdict::Swallow, Effect::Release(None))
            }
            _ => (Verdict::Pass, Effect::None),
        }
    }

    fn offset(&self, x: i32, y: i32) -> (f64, f64) {
        (f64::from(x - self.origin.0), f64::from(y - self.origin.1))
    }
}

/// Đổi độ lệch so với tâm menu thành hướng. Trục y của màn hình hướng xuống.
pub fn direction(dx: f64, dy: f64, cancel_radius: f64) -> Option<Dir> {
    if dx.hypot(dy) < cancel_radius {
        return None;
    }
    // atan2: 0° = Đông, 90° = Nam. Cộng 90° để 0 là Bắc, rồi chia thành 8 lát 45°.
    let deg = dy.atan2(dx).to_degrees();
    let index = ((deg + 90.0) / 45.0).round() as i32;
    Some(index.rem_euclid(8) as Dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g() -> Gesture {
        Gesture::new(Config::default())
    }

    #[test]
    fn eight_directions() {
        let r = 30.0;
        assert_eq!(direction(0.0, -50.0, r), Some(0));
        assert_eq!(direction(40.0, -40.0, r), Some(1));
        assert_eq!(direction(50.0, 0.0, r), Some(2));
        assert_eq!(direction(40.0, 40.0, r), Some(3));
        assert_eq!(direction(0.0, 50.0, r), Some(4));
        assert_eq!(direction(-40.0, 40.0, r), Some(5));
        assert_eq!(direction(-50.0, 0.0, r), Some(6));
        assert_eq!(direction(-40.0, -40.0, r), Some(7));
    }

    #[test]
    fn slice_edges_and_cancel_zone() {
        let r = 30.0;
        // Lệch 22° về phía Đông so với hướng Bắc vẫn là Bắc; lệch 23° đã sang Đông Bắc.
        let at = |deg: f64| {
            let a = (deg - 90.0).to_radians();
            direction(100.0 * a.cos(), 100.0 * a.sin(), r)
        };
        assert_eq!(at(22.0), Some(0));
        assert_eq!(at(23.0), Some(1));
        assert_eq!(at(-22.0), Some(0));
        assert_eq!(at(-23.0), Some(7));
        assert_eq!(direction(20.0, 0.0, r), None);
        assert_eq!(direction(0.0, 0.0, r), None);
    }

    #[test]
    fn dragging_far_closes_menu() {
        let mut g = g();
        g.right_down(500, 500, 1.0, false);
        assert!(matches!(g.mouse_move(520, 500, 1000), Effect::Open { .. }));
        // Ngay sau khi mở (cú vụt mở menu) chưa tính khoảng cách tự đóng.
        assert_eq!(g.mouse_move(1000, 500, 1100), Effect::Dir(Some(2)));
        // Đã qua thời gian chờ nhưng vẫn trong 450 px: giữ nguyên.
        assert_eq!(g.mouse_move(940, 500, 1300), Effect::None);
        assert_eq!(g.mouse_move(951, 500, 1310), Effect::Away);
        assert_eq!(g.phase(), Phase::Cancelled);
        // Kéo về lại cũng không mở lại; lần nhả bị nuốt, không chạy gì.
        assert_eq!(g.mouse_move(560, 500, 1400), Effect::None);
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::None));
    }

    #[test]
    fn quick_far_flick_still_selects() {
        // Nhấn, vụt thật xa, nhả ngay: vẫn chọn hướng chứ không bị đóng.
        let mut g = g();
        g.right_down(0, 0, 1.0, false);
        g.mouse_move(0, -20, u32::MAX - 50); // đồng hồ quay vòng cũng không sao
        assert_eq!(g.mouse_move(0, -900, 100), Effect::Dir(Some(0)));
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::Release(Some(0))));
    }

    #[test]
    fn close_distance_scales_with_dpi_and_can_be_off() {
        let mut g = g();
        g.right_down(0, 0, 2.0, false);
        g.mouse_move(30, 0, 0);
        assert_eq!(
            g.mouse_move(890, 0, 500),
            Effect::Dir(Some(2)),
            "2x: giới hạn là 900 px vật lý"
        );
        assert_eq!(g.mouse_move(910, 0, 510), Effect::Away);

        let mut off = Gesture::new(Config {
            close_distance_px: None,
            ..Config::default()
        });
        off.right_down(0, 0, 1.0, false);
        off.mouse_move(20, 0, 0);
        assert_eq!(off.mouse_move(5000, 0, 1000), Effect::Dir(Some(2)));
        assert_eq!(off.right_up(), (Verdict::Swallow, Effect::Release(Some(2))));
    }

    #[test]
    fn quick_click_replays_native_menu() {
        let mut g = g();
        assert_eq!(
            g.right_down(100, 100, 1.0, false),
            (Verdict::Swallow, Effect::None)
        );
        assert_eq!(g.mouse_move(105, 103, 0), Effect::None);
        assert_eq!(
            g.right_up(),
            (Verdict::Swallow, Effect::ReplayClick { x: 100, y: 100 })
        );
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn drag_opens_then_selects_direction() {
        let mut g = g();
        g.right_down(500, 500, 1.0, false);
        assert_eq!(
            g.mouse_move(489, 500, 0),
            Effect::Open {
                x: 500,
                y: 500,
                scale: 1.0,
                dir: None
            }
        );
        assert_eq!(g.mouse_move(470, 500, 0), Effect::Dir(Some(6)));
        assert_eq!(g.mouse_move(460, 502, 0), Effect::None);
        assert_eq!(g.mouse_move(470, 530, 0), Effect::Dir(Some(5)));
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::Release(Some(5))));
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn release_in_center_cancels() {
        let mut g = g();
        g.right_down(0, 0, 1.0, false);
        g.mouse_move(0, -60, 0);
        assert_eq!(g.mouse_move(0, -10, 0), Effect::Dir(None));
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::Release(None)));
    }

    #[test]
    fn threshold_scales_with_dpi() {
        let mut g = g();
        g.right_down(0, 0, 1.5, false);
        assert_eq!(g.mouse_move(12, 0, 0), Effect::None);
        assert!(matches!(g.mouse_move(16, 0, 0), Effect::Open { .. }));
        // Vùng huỷ cũng nhân 1.5: 40 px vẫn còn trong vùng huỷ 45 px.
        assert_eq!(g.mouse_move(40, 0, 0), Effect::None);
        assert_eq!(g.mouse_move(50, 0, 0), Effect::Dir(Some(2)));
    }

    #[test]
    fn escape_cancels_and_swallows_release() {
        let mut g = g();
        g.right_down(0, 0, 1.0, false);
        g.mouse_move(0, 80, 0);
        assert_eq!(g.escape(), (Verdict::Swallow, Effect::Release(None)));
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::None));
        assert_eq!(g.escape(), (Verdict::Pass, Effect::None));
    }

    #[test]
    fn escape_is_ignored_when_menu_closed() {
        let mut g = g();
        assert_eq!(g.escape(), (Verdict::Pass, Effect::None));
        g.right_down(0, 0, 1.0, false);
        assert_eq!(g.escape(), (Verdict::Pass, Effect::None));
    }

    #[test]
    fn other_button_hands_right_button_back() {
        let mut g = g();
        g.right_down(10, 20, 1.0, false);
        assert_eq!(
            g.other_button_down(Button::Left, 10, 20),
            (Verdict::Pass, Effect::ReplayDown { x: 10, y: 20 })
        );
        assert_eq!(g.mouse_move(200, 200, 0), Effect::None);
        assert_eq!(g.right_up(), (Verdict::Pass, Effect::None));
    }

    #[test]
    fn other_button_cancels_open_menu() {
        let mut g = g();
        g.right_down(0, 0, 1.0, false);
        g.mouse_move(80, 0, 0);
        assert_eq!(
            g.other_button_down(Button::Left, 80, 0),
            (Verdict::Pass, Effect::Release(None))
        );
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::None));
    }

    #[test]
    fn bypass_leaves_everything_to_windows() {
        let mut g = g();
        assert_eq!(g.right_down(0, 0, 1.0, true), (Verdict::Pass, Effect::None));
        assert_eq!(g.mouse_move(100, 0, 0), Effect::None);
        assert_eq!(g.right_up(), (Verdict::Pass, Effect::None));
    }

    #[test]
    fn release_without_press_passes() {
        let mut g = g();
        assert_eq!(g.right_up(), (Verdict::Pass, Effect::None));
    }

    /// Menu mở ở (0, 0), góc Bắc (0) có menu con.
    fn with_submenu_north() -> Gesture {
        let mut g = g();
        g.right_down(0, 0, 1.0, false);
        assert!(matches!(g.mouse_move(0, -20, 0), Effect::Open { .. }));
        g.set_submenus(1 << 0);
        g
    }

    #[test]
    fn dragging_past_the_edge_opens_submenu_in_place() {
        let mut g = with_submenu_north();
        assert_eq!(g.mouse_move(0, -100, 10), Effect::Dir(Some(0)));
        // Qua mép đĩa (165 px): mở menu con, góc cùng hướng vẫn đang được chỉ.
        assert_eq!(
            g.mouse_move(0, -170, 20),
            Effect::Drill {
                dir: 0,
                sticky: false
            }
        );
        // Trong menu con: kéo tiếp không mở thêm tầng nào, chỉ đổi hướng.
        assert_eq!(g.mouse_move(0, -300, 30), Effect::None);
        assert_eq!(g.mouse_move(200, -200, 40), Effect::Dir(Some(1)));
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::Release(Some(1))));
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn corners_without_submenu_never_drill() {
        let mut g = with_submenu_north();
        assert_eq!(g.mouse_move(300, 0, 10), Effect::Dir(Some(2)));
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::Release(Some(2))));
    }

    #[test]
    fn submenu_resets_close_timer_and_still_closes_when_far() {
        let mut g = with_submenu_north();
        g.mouse_move(0, -170, 1000); // mở menu con lúc 1000 ms
                                     // Ngay sau khi mở menu con, kéo xa chưa đóng.
        assert_eq!(g.mouse_move(0, -500, 1100), Effect::None);
        assert_eq!(g.mouse_move(0, -500, 1300), Effect::Away);
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::None));
    }

    #[test]
    fn releasing_on_submenu_corner_waits_for_a_click() {
        let mut g = with_submenu_north();
        g.mouse_move(0, -100, 10);
        assert_eq!(
            g.right_up(),
            (
                Verdict::Swallow,
                Effect::Drill {
                    dir: 0,
                    sticky: true
                }
            )
        );
        assert_eq!(g.phase(), Phase::Sticky);
        // Di chuột chọn góc Đông của menu con rồi bấm trái: chọn, nuốt cả nhấn lẫn nhả.
        assert_eq!(g.mouse_move(100, 0, 500), Effect::Dir(Some(2)));
        assert_eq!(
            g.other_button_down(Button::Left, 100, 0),
            (Verdict::Swallow, Effect::Release(Some(2)))
        );
        assert_eq!(g.other_button_up(Button::Left), Verdict::Swallow);
        assert_eq!(g.other_button_up(Button::Left), Verdict::Pass);
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn sticky_submenu_right_click_selects_and_swallows_release() {
        let mut g = with_submenu_north();
        g.mouse_move(0, -100, 10);
        g.right_up();
        g.mouse_move(-80, 0, 100);
        assert_eq!(
            g.right_down(-80, 0, 1.0, false),
            (Verdict::Swallow, Effect::Release(Some(6)))
        );
        assert_eq!(g.right_up(), (Verdict::Swallow, Effect::None));
    }

    #[test]
    fn sticky_submenu_click_in_center_cancels_and_outside_passes_through() {
        let mut g = with_submenu_north();
        g.mouse_move(0, -100, 10);
        g.right_up();
        g.mouse_move(5, 5, 100);
        assert_eq!(
            g.other_button_down(Button::Left, 5, 5),
            (Verdict::Swallow, Effect::Release(None))
        );

        let mut g = with_submenu_north();
        g.mouse_move(0, -100, 10);
        g.right_up();
        // Bấm hẳn ra ngoài menu: đóng menu, cú bấm vẫn tới app bên dưới.
        assert_eq!(
            g.other_button_down(Button::Left, 400, 0),
            (Verdict::Pass, Effect::Release(None))
        );
        assert_eq!(g.other_button_up(Button::Left), Verdict::Pass);
    }

    #[test]
    fn sticky_submenu_escape_and_timeout() {
        let mut g = with_submenu_north();
        g.mouse_move(0, -100, 10);
        g.right_up();
        assert_eq!(g.escape(), (Verdict::Swallow, Effect::Release(None)));
        assert_eq!(g.phase(), Phase::Idle);

        let mut g = with_submenu_north();
        g.mouse_move(0, -100, 10);
        g.right_up();
        assert_eq!(g.note_time(10 + STICKY_TIMEOUT_MS - 1), Effect::None);
        assert_eq!(
            g.note_time(10 + STICKY_TIMEOUT_MS + 1),
            Effect::Release(None)
        );
        assert_eq!(g.phase(), Phase::Idle);
    }

    #[test]
    fn new_press_clears_submenu_mask() {
        let mut g = with_submenu_north();
        g.right_up();
        g.right_down(0, 0, 1.0, false);
        g.mouse_move(0, -20, 0);
        // Mask của menu trước không còn: kéo qua mép Bắc chỉ chọn hướng, không mở menu con.
        assert_eq!(g.mouse_move(0, -200, 10), Effect::Dir(Some(0)));
    }

    #[test]
    fn new_press_resets_stuck_state() {
        let mut g = g();
        g.right_down(0, 0, 1.0, false);
        g.mouse_move(80, 0, 0);
        // Lỡ mất sự kiện nhả; lần nhấn tiếp theo bắt đầu lại sạch sẽ.
        assert_eq!(
            g.right_down(300, 300, 1.0, false),
            (Verdict::Swallow, Effect::None)
        );
        assert_eq!(
            g.right_up(),
            (Verdict::Swallow, Effect::ReplayClick { x: 300, y: 300 })
        );
    }
}
