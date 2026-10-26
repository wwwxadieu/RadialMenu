//! Hook WH_MOUSE_LL và WH_KEYBOARD_LL trên một thread có message loop riêng.
//!
//! Windows âm thầm gỡ hook nếu callback chạy quá lâu, nên callback chỉ cập nhật
//! máy trạng thái rồi đẩy việc sang thread khác qua channel:
//! - phát lại right-click → thread `radial-replay`
//! - báo cử chỉ cho app → `events`

use std::cell::RefCell;
use std::io;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEINPUT, MOUSE_EVENT_FLAGS, VK_ESCAPE, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetCursorPos, SetWindowsHookExW,
    UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_QUIT, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_SYSKEYDOWN, WM_XBUTTONDOWN,
};

use super::{BypassFn, SIGNATURE};
use crate::gesture::{Config, Effect, Gesture, Verdict};

/// Thread đang chạy hook, để gửi WM_QUIT khi dừng.
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
/// Nơi nhận hiệu ứng cử chỉ (app).
static EVENTS: OnceLock<Sender<Effect>> = OnceLock::new();
/// Hàng đợi phát lại right-click.
static REPLAY: OnceLock<Sender<Replay>> = OnceLock::new();

thread_local! {
    // Chỉ thread hook đụng tới máy trạng thái: LL hook luôn được gọi trên thread đã cài nó.
    static GESTURE: RefCell<Gesture> = RefCell::new(Gesture::new(Config::default()));
    // Quy tắc bỏ qua (app loại trừ, quyền Admin, toàn màn hình) do app cung cấp.
    static BYPASS: RefCell<Option<BypassFn>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, Copy)]
struct Replay {
    x: i32,
    y: i32,
    /// `true` = nhấn + nhả, `false` = chỉ nhấn.
    with_release: bool,
}

pub struct HookHandle {
    thread: Option<JoinHandle<()>>,
}

impl HookHandle {
    /// Gỡ hook và chờ thread kết thúc.
    pub fn stop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        let tid = HOOK_THREAD.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        let _ = thread.join();
    }
}

impl Drop for HookHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn start(cfg: Config, events: Sender<Effect>, bypass: BypassFn) -> io::Result<HookHandle> {
    if HOOK_THREAD.load(Ordering::SeqCst) != 0 {
        return Err(io::Error::other("Hook đã được cài"));
    }
    let _ = EVENTS.set(events);
    start_replay_thread()?;

    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    let thread = std::thread::Builder::new()
        .name("radial-hook".into())
        .spawn(move || run_hook_thread(cfg, bypass, ready_tx))?;

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(HookHandle {
            thread: Some(thread),
        }),
        Ok(Err(message)) => {
            let _ = thread.join();
            Err(io::Error::other(message))
        }
        Err(_) => Err(io::Error::other("Thread hook dừng trước khi sẵn sàng")),
    }
}

fn run_hook_thread(cfg: Config, bypass: BypassFn, ready: Sender<Result<(), String>>) {
    GESTURE.with(|g| g.borrow_mut().set_config(cfg));
    BYPASS.with(|b| *b.borrow_mut() = Some(bypass));

    unsafe {
        HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
        let module: Option<HINSTANCE> = GetModuleHandleW(None).ok().map(Into::into);

        let mouse = match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0) {
            Ok(h) => h,
            Err(e) => {
                HOOK_THREAD.store(0, Ordering::SeqCst);
                let _ = ready.send(Err(format!("Không cài được hook chuột: {e}")));
                return;
            }
        };
        let keyboard = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0) {
            Ok(h) => h,
            Err(e) => {
                let _ = UnhookWindowsHookEx(mouse);
                HOOK_THREAD.store(0, Ordering::SeqCst);
                let _ = ready.send(Err(format!("Không cài được hook bàn phím: {e}")));
                return;
            }
        };
        let _ = ready.send(Ok(()));

        // Hook LL chỉ được gọi khi thread này đang chờ trong GetMessage.
        // Không có cửa sổ nào, nên chỉ cần chờ tới khi nhận WM_QUIT (0) hoặc lỗi (-1).
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {}

        let _ = UnhookWindowsHookEx(keyboard);
        let _ = UnhookWindowsHookEx(mouse);
    }
    HOOK_THREAD.store(0, Ordering::SeqCst);
}

fn start_replay_thread() -> io::Result<()> {
    if REPLAY.get().is_some() {
        return Ok(());
    }
    let (tx, rx) = mpsc::channel::<Replay>();
    std::thread::Builder::new()
        .name("radial-replay".into())
        .spawn(move || {
            for job in rx {
                replay(job);
            }
        })?;
    let _ = REPLAY.set(tx);
    Ok(())
}

/// Phát lại cú nhấn chuột phải tại điểm nhấn ban đầu, có gắn SIGNATURE để hook bỏ qua.
fn replay(job: Replay) {
    let input = |flags: MOUSE_EVENT_FLAGS| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    };
    let inputs = [input(MOUSEEVENTF_RIGHTDOWN), input(MOUSEEVENTF_RIGHTUP)];
    let count = if job.with_release { 2 } else { 1 };
    unsafe {
        // Con trỏ chỉ lệch tối đa bằng ngưỡng kéo; đưa nó về đúng điểm nhấn để
        // app xác định đúng thứ được bấm (file, đoạn chữ…).
        let _ = SetCursorPos(job.x, job.y);
        SendInput(&inputs[..count], std::mem::size_of::<INPUT>() as i32);
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        // Bỏ qua sự kiện do phần mềm khác phát ra (điều khiển từ xa, macro) và của chính mình.
        let injected = info.flags & LLMHF_INJECTED != 0 || info.dwExtraInfo == SIGNATURE;
        if !injected && on_mouse(wparam.0 as u32, info.pt, info.time) == Verdict::Swallow {
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let message = wparam.0 as u32;
        if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
            let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            if info.vkCode == u32::from(VK_ESCAPE.0) {
                let (verdict, effect) = GESTURE.with(|g| g.borrow_mut().escape());
                dispatch(effect);
                if verdict == Verdict::Swallow {
                    return LRESULT(1);
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn on_mouse(message: u32, pt: POINT, time: u32) -> Verdict {
    let (verdict, effect) = GESTURE.with(|g| {
        let mut g = g.borrow_mut();
        match message {
            WM_RBUTTONDOWN => {
                if let Some(cfg) = super::take_pending_config() {
                    g.set_config(cfg);
                }
                let bypass = super::is_paused() || shift_down() || rule_bypass(pt);
                let scale = if bypass { 1.0 } else { monitor_scale(pt) };
                g.right_down(pt.x, pt.y, scale, bypass)
            }
            WM_MOUSEMOVE => (Verdict::Pass, g.mouse_move(pt.x, pt.y, time)),
            WM_RBUTTONUP => g.right_up(),
            WM_LBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
                (Verdict::Pass, g.other_button_down())
            }
            _ => (Verdict::Pass, Effect::None),
        }
    });
    dispatch(effect);
    verdict
}

fn dispatch(effect: Effect) {
    let replay_job = match effect {
        Effect::None => return,
        Effect::ReplayClick { x, y } => Some(Replay {
            x,
            y,
            with_release: true,
        }),
        Effect::ReplayDown { x, y } => Some(Replay {
            x,
            y,
            with_release: false,
        }),
        _ => None,
    };
    if let (Some(job), Some(tx)) = (replay_job, REPLAY.get()) {
        let _ = tx.send(job);
    }
    if let Some(tx) = EVENTS.get() {
        let _ = tx.send(effect);
    }
}

/// Hỏi app có muốn bỏ qua cú nhấn này không (chờ tối đa vài chục ms, xem `probe`).
fn rule_bypass(pt: POINT) -> bool {
    BYPASS.with(|b| b.borrow().as_ref().is_some_and(|f| f(pt.x, pt.y)))
}

fn shift_down() -> bool {
    // Bit cao = phím đang được giữ.
    unsafe { GetAsyncKeyState(i32::from(VK_SHIFT.0)) < 0 }
}

/// DPI của màn hình chứa điểm `pt`, chia cho 96 (scale 100%).
fn monitor_scale(pt: POINT) -> f64 {
    unsafe {
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
        match GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) {
            Ok(()) if dpi_x > 0 => f64::from(dpi_x) / 96.0,
            _ => 1.0,
        }
    }
}
