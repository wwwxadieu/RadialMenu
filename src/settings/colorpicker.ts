// Bộ chọn màu tự vẽ, thay hộp chọn màu của Windows (input type=color):
// ô bão hoà/độ sáng, thanh sắc màu, ô nhập mã HEX và nút lấy màu trên màn hình.
import { h, icon } from "./dom";
import { openPopover, place, type Popover } from "./controls";

interface Hsv {
  h: number; // 0–360
  s: number; // 0–1
  v: number; // 0–1
}

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));

function hsvToHex({ h, s, v }: Hsv): string {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
  };
  const to = (x: number) => Math.round(x * 255).toString(16).padStart(2, "0");
  return `#${to(f(5))}${to(f(3))}${to(f(1))}`.toUpperCase();
}

function hexToHsv(hex: string, keepHue: number): Hsv {
  const n = parseInt(hex.slice(1), 16);
  const r = ((n >> 16) & 255) / 255;
  const g = ((n >> 8) & 255) / 255;
  const b = (n & 255) / 255;
  const max = Math.max(r, g, b);
  const d = max - Math.min(r, g, b);
  let hue = keepHue; // màu xám không có sắc: giữ nguyên vị trí thanh sắc màu
  if (d > 0) {
    if (max === r) hue = 60 * (((g - b) / d) % 6);
    else if (max === g) hue = 60 * ((b - r) / d + 2);
    else hue = 60 * ((r - g) / d + 4);
    if (hue < 0) hue += 360;
  }
  return { h: hue, s: max === 0 ? 0 : d / max, v: max };
}

/** "abc", "#abc", "aabbcc", "#AABBCC" → "#AABBCC"; sai định dạng → null. */
export function normalizeHex(text: string): string | null {
  let t = text.trim().replace(/^#/, "");
  if (/^[0-9a-f]{3}$/i.test(t)) t = t.replace(/./g, (c) => c + c);
  return /^[0-9a-f]{6}$/i.test(t) ? `#${t.toUpperCase()}` : null;
}

interface EyeDropperResult {
  sRGBHex: string;
}
interface EyeDropperCtor {
  new (): { open(): Promise<EyeDropperResult> };
}

export interface ColorPickerOptions {
  value: string;
  /** Gọi liên tục khi đang kéo hoặc gõ: áp dụng thử ngay. */
  onInput: (hex: string) => void;
  /** Gọi khi khung đóng, với màu cuối cùng. */
  onClose: (hex: string) => void;
}

/** Mở bộ chọn màu bám theo `anchor`. */
export function openColorPicker(anchor: HTMLElement, o: ColorPickerOptions): Popover {
  let hsv = hexToHsv(normalizeHex(o.value) ?? "#7C83FF", 0);
  let hex = hsvToHex(hsv);

  const svThumb = h("span", { class: "cp-thumb" });
  const sv = h(
    "div",
    { class: "cp-sv", tabindex: "0", role: "slider", "aria-label": "Độ đậm và độ sáng", "aria-valuemin": "0", "aria-valuemax": "100" },
    svThumb,
  );
  const hueThumb = h("span", { class: "cp-thumb" });
  const hue = h(
    "div",
    { class: "cp-hue", tabindex: "0", role: "slider", "aria-label": "Sắc màu", "aria-valuemin": "0", "aria-valuemax": "360" },
    hueThumb,
  );
  const chip = h("span", { class: "cp-chip", "aria-hidden": "true" });
  const hexInput = h("input", {
    class: "cp-hex mono",
    type: "text",
    maxlength: "7",
    spellcheck: "false",
    "aria-label": "Mã màu HEX",
  });

  const EyeDropper = (window as unknown as { EyeDropper?: EyeDropperCtor }).EyeDropper;
  const dropper = EyeDropper
    ? h("button", { type: "button", class: "icon-btn cp-drop", title: "Lấy màu trên màn hình", "aria-label": "Lấy màu trên màn hình" }, icon("pipette", 16))
    : null;

  let isOpen = true;
  const pop = openPopover(anchor, "color-pop", () => {
    isOpen = false;
    anchor.setAttribute("aria-expanded", "false");
    o.onClose(hex);
  });
  anchor.setAttribute("aria-expanded", "true");
  pop.el.setAttribute("role", "dialog");
  pop.el.setAttribute("aria-label", "Chọn màu nhấn");
  pop.el.append(sv, hue, h("div", { class: "cp-row" }, chip, h("span", { class: "cp-hash", "aria-hidden": "true" }, "HEX"), hexInput, dropper));

  /** Vẽ lại theo `hsv`. `typed` = đang gõ trong ô HEX (đừng ghi đè chữ đang gõ). */
  const draw = (typed = false) => {
    sv.style.setProperty("--hue", `hsl(${hsv.h} 100% 50%)`);
    svThumb.style.left = `${hsv.s * 100}%`;
    svThumb.style.top = `${(1 - hsv.v) * 100}%`;
    svThumb.style.background = hex;
    hueThumb.style.left = `${(hsv.h / 360) * 100}%`;
    hueThumb.style.background = `hsl(${hsv.h} 100% 50%)`;
    chip.style.background = hex;
    sv.setAttribute("aria-valuenow", String(Math.round(hsv.s * 100)));
    sv.setAttribute("aria-valuetext", `độ đậm ${Math.round(hsv.s * 100)}%, độ sáng ${Math.round(hsv.v * 100)}%`);
    hue.setAttribute("aria-valuenow", String(Math.round(hsv.h)));
    if (!typed) {
      hexInput.value = hex;
      hexInput.classList.remove("bad");
    }
  };

  const commit = (next: Hsv, typed = false) => {
    hsv = next;
    const nextHex = hsvToHex(hsv);
    const changed = nextHex !== hex;
    hex = nextHex;
    draw(typed);
    if (changed) o.onInput(hex);
  };

  // Kéo trong ô bão hoà/độ sáng và trên thanh sắc màu.
  const drag = (area: HTMLElement, apply: (fx: number, fy: number) => void) => {
    const at = (e: PointerEvent) => {
      const r = area.getBoundingClientRect();
      apply(clamp((e.clientX - r.left) / r.width, 0, 1), clamp((e.clientY - r.top) / r.height, 0, 1));
    };
    area.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      area.focus();
      area.setPointerCapture(e.pointerId);
      at(e);
    });
    area.addEventListener("pointermove", (e) => {
      if (area.hasPointerCapture(e.pointerId)) at(e);
    });
  };
  drag(sv, (fx, fy) => commit({ h: hsv.h, s: fx, v: 1 - fy }));
  drag(hue, (fx) => commit({ ...hsv, h: Math.min(fx * 360, 359.9) }));

  // Phím mũi tên: bước nhỏ; giữ Shift: bước lớn.
  sv.addEventListener("keydown", (e) => {
    const step = e.shiftKey ? 0.1 : 0.01;
    const moves: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] };
    const m = moves[e.key];
    if (!m) return;
    e.preventDefault();
    commit({ h: hsv.h, s: clamp(hsv.s + m[0], 0, 1), v: clamp(hsv.v + m[1], 0, 1) });
  });
  hue.addEventListener("keydown", (e) => {
    const step = e.shiftKey ? 10 : 1;
    const d = e.key === "ArrowLeft" || e.key === "ArrowDown" ? -step : e.key === "ArrowRight" || e.key === "ArrowUp" ? step : 0;
    if (!d) return;
    e.preventDefault();
    commit({ ...hsv, h: (hsv.h + d + 360) % 360 });
  });

  // Ô HEX: áp dụng ngay khi gõ đủ mã hợp lệ; rời ô hoặc Enter thì chuẩn hoá lại.
  hexInput.addEventListener("input", () => {
    const parsed = normalizeHex(hexInput.value);
    hexInput.classList.toggle("bad", !parsed && hexInput.value.replace("#", "").length >= 6);
    if (parsed && hexInput.value.replace("#", "").length >= 6) commit(hexToHsv(parsed, hsv.h), true);
  });
  const settleHex = () => {
    const parsed = normalizeHex(hexInput.value);
    if (parsed) commit(hexToHsv(parsed, hsv.h));
    else draw();
  };
  hexInput.addEventListener("blur", settleHex);
  hexInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      settleHex();
      hexInput.select();
    }
  });

  dropper?.addEventListener("click", async () => {
    try {
      const { sRGBHex } = await new EyeDropper!().open();
      const parsed = normalizeHex(sRGBHex);
      if (!parsed) return;
      commit(hexToHsv(parsed, hsv.h));
      // Lúc lấy màu, cửa sổ mất focus nên khung có thể đã đóng: báo lại màu cuối.
      if (!isOpen) o.onClose(hex);
    } catch {
      // Người dùng bấm Esc khi đang lấy màu: không làm gì.
    }
  });

  // Esc ở bất kỳ đâu trong khung: đóng và trả focus về nút mở.
  pop.el.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      pop.close();
      anchor.focus();
    }
  });

  draw();
  place(pop.el, anchor);
  sv.focus();
  return pop;
}
