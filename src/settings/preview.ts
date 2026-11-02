// Bản xem trước menu tròn trong cửa sổ cài đặt: cùng hình học với menu thật,
// bấm (hoặc dùng phím mũi tên) vào một ô để chọn góc cần sửa.
import { arc, C, DIRS, point, R_ITEM, STAGE, wedge } from "../shared/geometry";
import { ICONS } from "../shared/icons";
import type { Profile } from "../shared/types";
import { h, icon, svgEl } from "./dom";

const DIR_INDEX: Record<string, number> = Object.fromEntries(DIRS.map((d, i) => [d.id, i]));

export interface PreviewOptions {
  /** Có cho bấm chọn góc không. */
  interactive: boolean;
  onPick?: (dir: number) => void;
  label?: string;
}

interface ItemView {
  icon: SVGPathElement;
  label: HTMLSpanElement;
  box: HTMLDivElement;
}

export class RadialPreview {
  readonly el: HTMLDivElement;
  private wedges: SVGPathElement[] = [];
  private items: ItemView[] = [];
  private arcPath: SVGPathElement;
  private hubIcon: SVGPathElement;
  private hubName: HTMLSpanElement;
  private hubSub: HTMLSpanElement;
  private selected: number | null = null;

  constructor(private opts: PreviewOptions) {
    this.el = h("div", { class: `radial${opts.interactive ? " interactive" : ""}` });
    const svg = svgEl("svg", { viewBox: `0 0 ${STAGE} ${STAGE}`, class: "radial-svg" });
    if (opts.label) {
      svg.setAttribute("role", "group");
      svg.setAttribute("aria-label", opts.label);
    } else {
      svg.setAttribute("aria-hidden", "true");
    }
    svg.append(svgEl("circle", { cx: C, cy: C, r: 172, class: "r-disc" }));

    for (let i = 0; i < 8; i++) {
      const p = svgEl("path", { d: wedge(i), class: "r-wedge", "data-i": i });
      if (opts.interactive) {
        p.setAttribute("tabindex", i === 0 ? "0" : "-1");
        p.setAttribute("role", "button");
        p.addEventListener("click", () => this.pick(i));
        p.addEventListener("keydown", (e) => this.onKey(e, i));
      }
      svg.append(p);
      this.wedges.push(p);
    }
    this.arcPath = svgEl("path", { class: "r-arc", d: arc(0, 59, 19) });
    svg.append(this.arcPath);

    for (let i = 0; i < 8; i++) {
      const [x, y] = point(R_ITEM, -90 + i * 45);
      const fo = svgEl("foreignObject", { x: x - 43, y: y - 35, width: 86, height: 70, class: "r-fo" });
      const ico = icon("", 20);
      const label = h("span", { class: "r-label" });
      const box = h("div", { class: "r-item" }, ico, label);
      fo.append(box);
      svg.append(fo);
      this.items.push({ icon: ico.firstElementChild as SVGPathElement, label, box });
    }

    svg.append(svgEl("circle", { cx: C, cy: C, r: 56, class: "r-hub" }));
    const hubFo = svgEl("foreignObject", { x: C - 56, y: C - 56, width: 112, height: 112, class: "r-fo" });
    const hubIco = icon("grid", 22);
    this.hubIcon = hubIco.firstElementChild as SVGPathElement;
    this.hubName = h("span", { class: "r-hub-name" });
    this.hubSub = h("span", { class: "r-hub-sub" });
    hubFo.append(h("div", { class: "r-hub-box" }, hubIco, this.hubName, this.hubSub));
    svg.append(hubFo);

    // Lớp kính mờ dưới đĩa (SVG không tự làm mờ nền được).
    this.el.append(h("div", { class: "radial-glass", "aria-hidden": "true" }), svg);
  }

  update(profile: Profile, selected: number | null) {
    this.selected = selected;
    const slots: (Profile["slots"][number] | null)[] = Array(8).fill(null);
    for (const s of profile.slots) {
      const i = DIR_INDEX[s.dir];
      if (i !== undefined) slots[i] = s;
    }
    slots.forEach((slot, i) => {
      const item = this.items[i];
      const on = i === selected;
      item.box.classList.toggle("empty", !slot);
      item.box.classList.toggle("on", on);
      item.icon.setAttribute("d", slot ? (ICONS[slot.icon] ?? ICONS.smile) : ICONS.plus);
      item.label.textContent = slot ? slot.label || "Chưa đặt tên" : "Trống";
      const w = this.wedges[i];
      w.classList.toggle("on", on);
      w.classList.toggle("empty", !slot);
      if (this.opts.interactive) {
        const d = DIRS[i];
        w.setAttribute("aria-label", `Góc ${d.name}: ${slot ? slot.label || "chưa đặt tên" : "trống"}`);
        w.setAttribute("aria-pressed", String(on));
        w.setAttribute("tabindex", i === (selected ?? 0) ? "0" : "-1");
      }
    });
    this.arcPath.classList.toggle("on", selected !== null);
    if (selected !== null) this.arcPath.setAttribute("d", arc(selected, 59, 19));

    this.hubIcon.setAttribute("d", ICONS[profile.icon] ?? ICONS.grid);
    this.hubName.textContent = profile.name || "Chưa đặt tên";
    this.hubSub.textContent = profile.match.process[0] ?? (profile.id === "default" ? "mọi app khác" : "");
  }

  /** Góc nằm dưới điểm (clientX, clientY), tính theo hình học của đĩa; ngoài đĩa hoặc trong tâm = null. */
  dirAt(x: number, y: number): number | null {
    const r = this.el.getBoundingClientRect();
    if (!r.width) return null;
    const unit = r.width / STAGE;
    const dx = (x - (r.left + r.width / 2)) / unit;
    const dy = (y - (r.top + r.height / 2)) / unit;
    const dist = Math.hypot(dx, dy);
    if (dist < 56 || dist > 190) return null;
    const deg = (Math.atan2(dy, dx) * 180) / Math.PI + 90; // 0 = Bắc, theo chiều kim đồng hồ
    return ((Math.round(deg / 45) % 8) + 8) % 8;
  }

  /** Tô sáng góc sắp được thả lệnh vào (kéo từ Thư viện lệnh). */
  setDrop(i: number | null) {
    this.el.classList.toggle("dropping", i !== null);
    this.wedges.forEach((w, j) => w.classList.toggle("drop", j === i));
    this.items.forEach((it, j) => it.box.classList.toggle("drop", j === i));
  }

  /** Nháy nhẹ một góc vừa được gán lệnh. */
  flash(i: number) {
    const w = this.wedges[i];
    if (!w) return;
    w.classList.remove("flash");
    void w.getBoundingClientRect();
    w.classList.add("flash");
    window.setTimeout(() => w.classList.remove("flash"), 700);
  }

  focusDir(i: number) {
    this.wedges[i]?.focus();
  }

  private pick(i: number) {
    this.opts.onPick?.(i);
  }

  private onKey(e: KeyboardEvent, i: number) {
    let next: number | null = null;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      this.pick(i);
      return;
    }
    if (e.key === "ArrowRight" || e.key === "ArrowDown") next = (i + 1) % 8;
    if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = (i + 7) % 8;
    if (next !== null) {
      e.preventDefault();
      this.pick(next);
      requestAnimationFrame(() => this.focusDir(next));
    }
  }

  get current() {
    return this.selected;
  }
}
