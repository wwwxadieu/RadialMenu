// Thư viện lệnh: các lệnh hay dùng, soạn sẵn. Kéo một lệnh thả vào một góc trên hình
// menu để gán, hoặc bấm (Enter) để gán vào góc đang chọn. Kéo bằng sự kiện con trỏ
// (không dùng kéo-thả HTML) để chạy ổn trong WebView2.
import { prettyChord } from "../shared/keys";
import type { Action } from "../shared/types";
import { h, icon } from "./dom";

export interface Preset {
  label: string;
  icon: string;
  action: Action;
  clickFirst?: boolean;
}

interface Group {
  name: string;
  items: Preset[];
}

const hk = (label: string, ico: string, ...keys: string[]): Preset => ({ label, icon: ico, action: { type: "hotkey", keys } });
/** Phím tắt cần chọn đúng thứ dưới con trỏ trước (click vào chỗ nhấn). */
const hkc = (label: string, ico: string, ...keys: string[]): Preset => ({ ...hk(label, ico, ...keys), clickFirst: true });
const open = (label: string, ico: string, url: string): Preset => ({ label, icon: ico, action: { type: "url", url } });
const run = (label: string, ico: string, cmd: string, ...args: string[]): Preset => ({
  label,
  icon: ico,
  action: args.length ? { type: "command", cmd, args } : { type: "command", cmd },
});

export const LIBRARY: Group[] = [
  {
    name: "Chỉnh sửa",
    items: [
      hk("Sao chép", "copy", "Ctrl", "C"),
      hk("Cắt", "scissors", "Ctrl", "X"),
      hk("Dán", "paste", "Ctrl", "V"),
      hk("Hoàn tác", "undo", "Ctrl", "Z"),
      hk("Làm lại", "redo", "Ctrl", "Y"),
      hk("Chọn tất cả", "textCursor", "Ctrl", "A"),
      hk("Tìm", "search", "Ctrl", "F"),
      hk("Lưu", "save", "Ctrl", "S"),
    ],
  },
  {
    name: "Trình duyệt",
    items: [
      hk("Tab mới", "plus", "Ctrl", "T"),
      hk("Đóng tab", "x", "Ctrl", "W"),
      hk("Mở lại tab", "undo", "Ctrl", "Shift", "T"),
      hk("Tải lại", "refresh", "Ctrl", "R"),
      hk("Quay lại", "arrowLeft", "Alt", "Left"),
      hk("Tiến", "arrowRight", "Alt", "Right"),
      hk("Tab ẩn danh", "eyeOff", "Ctrl", "Shift", "N"),
      hk("Tab kế tiếp", "columns", "Ctrl", "Tab"),
      hk("Phóng to trang", "zoomIn", "Ctrl", "="),
      hk("Đánh dấu trang", "bookmark", "Ctrl", "D"),
      hk("Tải xuống", "download", "Ctrl", "J"),
    ],
  },
  {
    name: "Cửa sổ",
    items: [
      hk("Đóng cửa sổ", "x", "Alt", "F4"),
      hk("Thu nhỏ", "minimize", "Win", "Down"),
      hk("Phóng to cửa sổ", "maximize", "Win", "Up"),
      hk("Cửa sổ kế", "windows", "Alt", "Tab"),
      hk("Xem mọi cửa sổ", "grid", "Win", "Tab"),
      hk("Hiện Desktop", "monitor", "Win", "D"),
      hk("Xếp sang trái", "arrowLeft", "Win", "Left"),
      hk("Xếp sang phải", "arrowRight", "Win", "Right"),
    ],
  },
  {
    name: "Windows",
    items: [
      hk("Chụp vùng", "crop", "Win", "Shift", "S"),
      hk("Clipboard", "clipboard", "Win", "V"),
      hk("Emoji", "smile", "Win", "."),
      hk("Tìm kiếm", "search", "Win", "S"),
      hk("File Explorer", "folder", "Win", "E"),
      hk("Cài đặt Windows", "settings", "Win", "I"),
      hk("Task Manager", "activity", "Ctrl", "Shift", "Esc"),
      // Win+L không gửi giả lập được (Windows chặn), nên gọi thẳng lệnh khoá máy.
      run("Khoá máy", "lock", "rundll32.exe", "user32.dll,LockWorkStation"),
    ],
  },
  {
    name: "Âm thanh & media",
    items: [
      hk("Phát/Dừng", "play", "MediaPlay"),
      hk("Bài sau", "arrowRight", "MediaNext"),
      hk("Bài trước", "arrowLeft", "MediaPrev"),
      hk("Tắt tiếng", "volumeX", "VolumeMute"),
      hk("Tăng âm lượng", "volume", "VolumeUp"),
      hk("Giảm âm lượng", "volume", "VolumeDown"),
    ],
  },
  {
    name: "Thư mục & ứng dụng",
    items: [
      open("Thư mục Tải về", "download", "%DOWNLOADS%"),
      open("Thư mục Tài liệu", "folder", "%DOCUMENTS%"),
      open("Thư mục Desktop", "monitor", "%DESKTOP%"),
      run("Terminal", "terminal", "wt"),
      run("Notepad", "pencil", "notepad.exe"),
      run("Máy tính", "calculator", "calc.exe"),
      { label: "Cài đặt Radial Menu", icon: "settings", action: { type: "settings" } },
    ],
  },
  {
    name: "File Explorer",
    items: [
      hk("Thư mục mới", "folderPlus", "Ctrl", "Shift", "N"),
      hkc("Đổi tên", "pencil", "F2"),
      hkc("Xoá", "trash", "Delete"),
      hkc("Thuộc tính", "info", "Alt", "Enter"),
      hkc("Copy đường dẫn", "pathCopy", "Ctrl", "Shift", "C"),
      hk("Lên thư mục cha", "arrowUp", "Alt", "Up"),
    ],
  },
  {
    name: "Code editor",
    items: [
      hk("Định dạng", "format", "Shift", "Alt", "F"),
      hk("Comment", "comment", "Ctrl", "/"),
      hkc("Tới định nghĩa", "goto", "F12"),
      hkc("Đổi tên biến", "textCursor", "F2"),
      hk("Mở file nhanh", "search", "Ctrl", "P"),
      hk("Bảng lệnh", "keyboard", "Ctrl", "Shift", "P"),
      hk("Terminal", "terminal", "Ctrl", "`"),
      hk("Chạy", "play", "F5"),
      hk("Chia đôi", "columns", "Ctrl", "\\"),
    ],
  },
];

/** Mô tả ngắn hiện bên phải mỗi lệnh. */
export function presetHint(a: Action): string {
  switch (a.type) {
    case "hotkey":
      return prettyChord(a.keys);
    case "command":
      return a.cmd.replace(/\.exe$/i, "");
    case "url":
      return a.url;
    case "settings":
      return "Mở cài đặt";
    default:
      return "";
  }
}

/** Bỏ dấu để tìm "sao chep" vẫn ra "Sao chép". */
const fold = (s: string) =>
  s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/đ/g, "d");

export interface LibraryHost {
  /** Góc đang chọn (bấm một lệnh thì gán vào đây). */
  selected(): number;
  /** Tên góc để ghi trong lời nhắc, ví dụ "Đông". */
  dirName(i: number): string;
  /** Góc nằm dưới điểm (clientX, clientY) trên hình menu, hoặc null. */
  dirAt(x: number, y: number): number | null;
  /** Tô sáng góc sắp thả vào (null = bỏ tô). */
  hover(dir: number | null): void;
  assign(dir: number, preset: Preset): void;
}

const DRAG_START_PX = 5;

export function libraryPanel(host: LibraryHost): { el: HTMLElement; refresh(): void } {
  const search = h("input", { type: "search", class: "input lib-search", placeholder: "Tìm lệnh, ví dụ: sao chép, chụp, tab…", "aria-label": "Tìm lệnh trong thư viện" });
  const hint = h("p", { class: "hint lib-hint" });
  const list = h("div", { class: "lib-list", role: "list" });

  const drawHint = () => {
    hint.replaceChildren(
      "Kéo một lệnh thả vào góc trên hình menu. Hoặc bấm vào lệnh để gán cho góc đang chọn (",
      h("b", null, host.dirName(host.selected())),
      ").",
    );
  };

  const item = (p: Preset): HTMLButtonElement => {
    const btn = h(
      "button",
      { type: "button", class: "lib-item", role: "listitem", title: `Kéo vào một góc, hoặc bấm để gán cho góc đang chọn` },
      icon("grip", 14, "ico lib-grip"),
      icon(p.icon, 17, "ico lib-ico"),
      h("span", { class: "lib-label" }, p.label),
      h("span", { class: "lib-keys mono" }, presetHint(p.action)),
    );
    attachDrag(btn, p, host);
    btn.addEventListener("click", () => {
      if (btn.dataset.dragged === "1") {
        delete btn.dataset.dragged;
        return;
      }
      host.assign(host.selected(), p);
    });
    return btn;
  };

  const draw = () => {
    const q = fold(search.value.trim());
    const groups = LIBRARY.map((g) => ({
      name: g.name,
      items: q ? g.items.filter((p) => fold(`${p.label} ${g.name} ${presetHint(p.action)}`).includes(q)) : g.items,
    })).filter((g) => g.items.length);
    list.replaceChildren(
      ...groups.flatMap((g) => [h("div", { class: "lib-group", role: "presentation" }, g.name), ...g.items.map(item)]),
    );
    if (!groups.length) list.append(h("p", { class: "muted lib-empty" }, "Không có lệnh nào khớp. Bấm sang “Sửa góc” để tự đặt phím tắt hay lệnh."));
  };

  search.addEventListener("input", draw);
  drawHint();
  draw();
  const el = h("div", { class: "lib" }, h("div", { class: "lib-search-wrap" }, icon("search", 16, "ico lib-search-ico"), search), hint, list);
  // refresh: góc đang chọn đổi, cập nhật lời nhắc.
  return { el, refresh: drawHint };
}

/** Kéo một lệnh: bóng lệnh chạy theo con trỏ, góc dưới con trỏ sáng lên, thả để gán. */
function attachDrag(btn: HTMLButtonElement, preset: Preset, host: LibraryHost) {
  btn.addEventListener("pointerdown", (down) => {
    if (down.button !== 0) return;
    const startX = down.clientX;
    const startY = down.clientY;
    let ghost: HTMLElement | null = null;
    let target: number | null = null;

    const move = (e: PointerEvent) => {
      if (!ghost) {
        if (Math.hypot(e.clientX - startX, e.clientY - startY) < DRAG_START_PX) return;
        ghost = h("div", { class: "lib-ghost", "aria-hidden": "true" }, icon(preset.icon, 17), h("span", null, preset.label));
        document.body.append(ghost);
        document.body.classList.add("lib-dragging");
        btn.classList.add("lifted");
        btn.setPointerCapture(e.pointerId);
      }
      ghost.style.transform = `translate(${e.clientX + 14}px, ${e.clientY + 10}px)`;
      const next = host.dirAt(e.clientX, e.clientY);
      if (next !== target) {
        target = next;
        host.hover(target);
        ghost.classList.toggle("over", target !== null);
      }
    };

    const end = (drop: boolean) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("keydown", esc, true);
      if (!ghost) return;
      btn.dataset.dragged = "1"; // để lần click ngay sau lần thả không gán thêm lần nữa
      window.setTimeout(() => delete btn.dataset.dragged, 0);
      ghost.remove();
      document.body.classList.remove("lib-dragging");
      btn.classList.remove("lifted");
      host.hover(null);
      if (drop && target !== null) host.assign(target, preset);
    };
    const up = () => end(true);
    const cancel = () => end(false);
    const esc = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      end(false);
    };

    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("keydown", esc, true);
  });
}
