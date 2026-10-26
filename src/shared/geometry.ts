// Hình học của menu tròn, trên sân khấu 360×360 (tâm 180, 180). Dùng chung cho menu
// thật và bản xem trước trong cửa sổ cài đặt, để hai bên luôn trùng khít.

export const STAGE = 360;
export const C = STAGE / 2;
/** Bán kính ngoài / trong của 8 ô. */
export const R_OUT = 170;
export const R_IN = 62;
/** Bán kính đặt icon + nhãn của từng ô. */
export const R_ITEM = 116;

export function point(r: number, deg: number): [number, number] {
  const a = (deg * Math.PI) / 180;
  return [C + r * Math.cos(a), C + r * Math.sin(a)];
}

const f = (n: number) => n.toFixed(2);

/** Góc giữa của hướng i: 0 = Bắc, theo chiều kim đồng hồ. */
export const angleOf = (i: number) => -90 + i * 45;

/** Đường viền ô thứ i (hình quạt khuyết). */
export function wedge(i: number, outer = R_OUT, inner = R_IN, pad = 1.1): string {
  const mid = angleOf(i);
  const [ax, ay] = point(outer, mid - 22.5 + pad);
  const [bx, by] = point(outer, mid + 22.5 - pad);
  const [cx, cy] = point(inner, mid + 22.5 - pad);
  const [dx, dy] = point(inner, mid - 22.5 + pad);
  return `M${f(ax)} ${f(ay)}A${outer} ${outer} 0 0 1 ${f(bx)} ${f(by)}L${f(cx)} ${f(cy)}A${inner} ${inner} 0 0 0 ${f(dx)} ${f(dy)}Z`;
}

/** Cung tròn quanh hướng i, rộng ±span độ. */
export function arc(i: number, rad: number, span: number): string {
  const mid = angleOf(i);
  const [ax, ay] = point(rad, mid - span);
  const [bx, by] = point(rad, mid + span);
  return `M${f(ax)} ${f(ay)}A${rad} ${rad} 0 0 1 ${f(bx)} ${f(by)}`;
}

export const DIRS: ReadonlyArray<{ id: string; name: string; arrow: string }> = [
  { id: "N", name: "Bắc", arrow: "↑" },
  { id: "NE", name: "Đông Bắc", arrow: "↗" },
  { id: "E", name: "Đông", arrow: "→" },
  { id: "SE", name: "Đông Nam", arrow: "↘" },
  { id: "S", name: "Nam", arrow: "↓" },
  { id: "SW", name: "Tây Nam", arrow: "↙" },
  { id: "W", name: "Tây", arrow: "←" },
  { id: "NW", name: "Tây Bắc", arrow: "↖" },
];
