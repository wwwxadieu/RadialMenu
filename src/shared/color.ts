// Màu nhấn: tính màu chữ đặt trên nền màu nhấn và biến CSS dùng chung.

export function hexToRgb(hex: string): [number, number, number] {
  const h = hex.trim().replace("#", "");
  if (!/^[0-9a-fA-F]{6}$/.test(h)) return [96, 205, 255];
  const n = parseInt(h, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function luminance([r, g, b]: [number, number, number]): number {
  const lin = (v: number) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/** Chữ trên nền màu nhấn: tối nếu màu nhấn sáng, trắng nếu màu nhấn đậm. */
export function inkFor(hex: string): string {
  return luminance(hexToRgb(hex)) > 0.19 ? "#06121A" : "#FFFFFF";
}

/** Gán --accent, --accent-rgb, --accent-ink lên một phần tử. */
export function applyAccent(el: HTMLElement, hex: string) {
  el.style.setProperty("--accent", hex);
  el.style.setProperty("--accent-rgb", hexToRgb(hex).join(", "));
  el.style.setProperty("--accent-ink", inkFor(hex));
}
