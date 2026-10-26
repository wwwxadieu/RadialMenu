// Trang sửa một profile: tên, icon, app áp dụng, và lệnh của 8 góc.
// Mọi thay đổi nằm trong bản nháp (`Draft`) tới khi bấm Lưu.
import { DIRS } from "../shared/geometry";
import type { AppState, DirId, Profile, Slot } from "../shared/types";
import { actionFields, defaultAction, groupName, isPaste, TOP_TYPES, type ActionHost } from "./action-fields";
import { checkbox } from "./controls";
import { chip, chipAdder, h, icon, segmented, toggle } from "./dom";
import { iconPicker } from "./overlay";
import { RadialPreview } from "./preview";

export interface Draft {
  /** Mã trước khi sửa; `null` = profile mới chưa lưu. */
  originalId: string | null;
  profile: Profile;
  /** JSON của bản đã lưu, để biết có thay đổi chưa. */
  baseline: string;
  /** Góc đang sửa. */
  selected: number;
}

export interface EditorHost {
  app(): AppState;
  /** App đang mở + app gần đây, gợi ý khi thêm chương trình. */
  suggestions(): Promise<{ value: string; note: string }[]>;
  /** Bản nháp đổi (cập nhật thanh Lưu, danh sách bên trái). */
  changed(): void;
  remove(): void;
  resetToBuiltin(): void;
}

export const isDirty = (d: Draft) => JSON.stringify(d.profile) !== d.baseline;

export function makeDraft(profile: Profile, isNew = false): Draft {
  const copy: Profile = structuredClone(profile);
  copy.match.class ??= [];
  return { originalId: isNew ? null : profile.id, profile: copy, baseline: isNew ? "" : JSON.stringify(copy), selected: 0 };
}

const slotAt = (p: Profile, i: number) => p.slots.find((s) => s.dir === DIRS[i].id) ?? null;

export class ProfileEditor {
  readonly el: HTMLElement;
  private preview: RadialPreview;
  private slotBox = h("section", { class: "card slot-card", "aria-labelledby": "slot-title" });
  private headBox = h("section", { class: "card head-card", "aria-label": "Thông tin profile" });
  /** Hiện ô thêm lớp cửa sổ (tuỳ chọn nâng cao). */
  private advanced = false;

  constructor(private draft: Draft, private host: EditorHost) {
    this.preview = new RadialPreview({
      interactive: true,
      label: "Xem trước menu, chọn góc cần sửa",
      onPick: (i) => {
        this.draft.selected = i;
        this.preview.update(this.draft.profile, i);
        this.renderSlot();
      },
    });
    const previewCard = h(
      "section",
      { class: "card preview-card", "aria-label": "Xem trước menu" },
      h("p", { class: "card-note" }, "Bấm vào một góc để sửa lệnh của góc đó"),
      this.preview.el,
    );
    this.el = h("div", { class: "page profile-page" }, this.headBox, h("div", { class: "editor-grid" }, previewCard, this.slotBox));
    this.renderHead();
    this.renderSlot();
    this.preview.update(draft.profile, draft.selected);
  }

  /** Bản nháp đổi: vẽ lại phần xem trước và báo ra ngoài. */
  private touched() {
    this.preview.update(this.draft.profile, this.draft.selected);
    this.host.changed();
  }

  // ---------- Phần đầu: tên, icon, app áp dụng ----------

  private renderHead() {
    const p = this.draft.profile;
    const isDefault = p.id === "default";
    const builtin = this.host.app().builtins.some((b) => b.id === p.id);

    const iconBtn = h("button", { type: "button", class: "icon-btn lg accent", "aria-label": "Đổi icon của profile", title: "Đổi icon" }, icon(p.icon || "grid", 22));
    iconBtn.addEventListener("click", () =>
      iconPicker(iconBtn, p.icon, (name) => {
        p.icon = name;
        iconBtn.replaceChildren(icon(name, 22));
        this.touched();
      }),
    );

    const name = h("input", { type: "text", class: "title-input", value: p.name, "aria-label": "Tên profile", placeholder: "Tên profile", maxlength: 60 });
    name.addEventListener("input", () => {
      p.name = name.value;
      this.touched();
    });

    const tools = h("div", { class: "head-tools" });
    tools.append(
      toggle("Bật profile này", isDefault || p.enabled, (v) => {
        p.enabled = v;
        this.touched();
      }, undefined, isDefault),
    );
    if (builtin) {
      tools.append(h("button", { type: "button", class: "btn ghost sm", onclick: () => this.host.resetToBuiltin() }, icon("undo", 15), "Về mặc định"));
    }
    if (!isDefault) {
      tools.append(h("button", { type: "button", class: "btn ghost sm danger-text", onclick: () => this.host.remove() }, icon("trash", 15), "Xoá"));
    }

    this.headBox.replaceChildren(h("div", { class: "head-row" }, iconBtn, name, tools));
    if (isDefault) {
      this.headBox.append(h("p", { class: "sub" }, "Dùng cho mọi app chưa có profile riêng, và cho app có profile đang tắt."));
    } else {
      this.headBox.append(this.matchRow());
    }
  }

  private matchRow(): HTMLElement {
    const p = this.draft.profile;
    const classes = (p.match.class ??= []);
    const row = h("div", { class: "match" });
    const chips = h("div", { class: "chips", role: "list", "aria-label": "Chương trình áp dụng" });

    const redraw = () => {
      const fresh = this.matchRow();
      row.replaceWith(fresh);
    };

    p.match.process.forEach((proc, i) => {
      chips.append(chip(proc, () => {
        p.match.process.splice(i, 1);
        this.touched();
        redraw();
      }));
    });
    classes.forEach((cls, i) => {
      const c = chip(`cửa sổ: ${cls}`, () => {
        classes.splice(i, 1);
        this.touched();
        redraw();
      });
      c.classList.add("alt");
      c.title = "Chỉ áp dụng cho cửa sổ có lớp này";
      chips.append(c);
    });

    chips.append(
      chipAdder("+ Thêm chương trình", "notepad.exe", (v) => {
        const value = v.toLowerCase();
        if (!p.match.process.includes(value)) p.match.process.push(value);
        this.touched();
        redraw();
      }, () => this.host.suggestions()),
    );
    if (classes.length || this.advanced) {
      chips.append(
        chipAdder("+ Lớp cửa sổ", "CabinetWClass", (value) => {
          if (!classes.some((c) => c.toLowerCase() === value.toLowerCase())) classes.push(value);
          this.touched();
          redraw();
        }),
      );
    } else {
      chips.append(
        h("button", {
          type: "button",
          class: "link-btn",
          title: "Chỉ áp dụng cho một loại cửa sổ của chương trình, ví dụ Desktop của explorer.exe",
          onclick: () => {
            this.advanced = true;
            redraw();
          },
        }, "Lọc theo lớp cửa sổ"),
      );
    }

    row.append(h("span", { class: "match-label" }, "Áp dụng cho"), chips);
    const warn = this.conflicts();
    if (warn) row.append(h("p", { class: "hint warn" }, warn));
    if (!p.match.process.length && !classes.length) row.append(h("p", { class: "hint warn" }, "Cần ít nhất một chương trình, ví dụ notepad.exe."));
    return row;
  }

  /** Cảnh báo khi một chương trình đã thuộc profile khác (cùng điều kiện lớp cửa sổ). */
  private conflicts(): string | null {
    const p = this.draft.profile;
    const mine = p.match.class ?? [];
    for (const other of this.host.app().profiles) {
      if (other.id === (this.draft.originalId ?? p.id) || !other.enabled) continue;
      const theirs = other.match.class ?? [];
      if (mine.length !== theirs.length || !mine.every((c) => theirs.some((t) => t.toLowerCase() === c.toLowerCase()))) continue;
      const shared = p.match.process.find((x) => other.match.process.includes(x));
      if (shared) return `${shared} cũng có trong profile "${other.name}". Hai profile khớp như nhau thì không chắc profile nào được dùng.`;
    }
    return null;
  }

  // ---------- Góc đang chọn ----------

  private renderSlot() {
    const p = this.draft.profile;
    const i = this.draft.selected;
    const d = DIRS[i];
    const slot = slotAt(p, i);
    const title = h("h2", { id: "slot-title" }, slot ? slot.label || "Chưa đặt tên" : "Góc trống");
    const head = h("div", { class: "slot-head" }, h("span", { class: "eyebrow" }, `GÓC ${d.name.toUpperCase()} ${d.arrow}`), title);

    if (!slot) {
      this.slotBox.replaceChildren(
        head,
        h("p", { class: "muted" }, "Nhả chuột ở góc này sẽ không làm gì."),
        h("button", {
          type: "button",
          class: "btn primary",
          onclick: () => {
            p.slots.push({ dir: d.id as DirId, label: "", icon: "star", action: defaultAction("hotkey") });
            p.slots.sort((a, b) => DIRS.findIndex((x) => x.id === a.dir) - DIRS.findIndex((x) => x.id === b.dir));
            this.touched();
            this.renderSlot();
            (this.slotBox.querySelector(".label-input") as HTMLInputElement | null)?.focus();
          },
        }, icon("plus", 16), "Thêm lệnh cho góc này"),
      );
      return;
    }

    const host: ActionHost = {
      changed: () => this.touched(),
      restructure: () => this.renderSlot(),
    };

    const iconBtn = h("button", { type: "button", class: "icon-btn", "aria-label": "Đổi icon của góc này", title: "Đổi icon" }, icon(slot.icon || "star", 20));
    iconBtn.addEventListener("click", () =>
      iconPicker(iconBtn, slot.icon, (name) => {
        slot.icon = name;
        iconBtn.replaceChildren(icon(name, 20));
        this.touched();
      }),
    );
    const label = h("input", { type: "text", class: "input label-input", value: slot.label, placeholder: "Ví dụ: Tab mới", maxlength: 24, id: "slot-label" });
    label.addEventListener("input", () => {
      slot.label = label.value;
      title.textContent = slot.label || "Chưa đặt tên";
      this.touched();
    });

    const types = segmented(groupName(), "Loại hành động", TOP_TYPES, slot.action.type, (t) => {
      slot.action = defaultAction(t);
      this.touched();
      this.renderSlot();
    });

    const click = checkbox(
      "Click vào chỗ vừa nhấn trước khi chạy",
      "Để chọn đúng file, chữ hay đối tượng dưới con trỏ (Đổi tên, Xoá, Tới định nghĩa…).",
      Boolean(slot.clickFirst),
      (on) => {
        if (on) slot.clickFirst = true;
        else delete slot.clickFirst;
        this.touched();
      },
    );

    const clip = checkbox(
      "Chỉ bật khi clipboard có dữ liệu",
      "Clipboard trống thì góc này hiện mờ và không chọn được. Góc Dán (Ctrl+V) tự bật sẵn.",
      (slot.when ?? (isPaste(slot.action) ? "clipboard" : "always")) === "clipboard",
      (on) => {
        const want = on ? "clipboard" : "always";
        // Trùng với cách tự đoán thì khỏi ghi vào file.
        if (want === (isPaste(slot.action) ? "clipboard" : "always")) delete slot.when;
        else slot.when = want;
        this.touched();
      },
    );

    this.slotBox.replaceChildren(
      head,
      h("div", { class: "field" }, h("label", { for: "slot-label" }, "Tên hiển thị"), h("div", { class: "label-row" }, iconBtn, label)),
      types,
      actionFields(slot.action, host),
      h("div", { class: "checks" }, click, clip),
      h(
        "div",
        { class: "slot-foot" },
        h("button", {
          type: "button",
          class: "btn ghost",
          onclick: () => {
            p.slots = p.slots.filter((s: Slot) => s !== slot);
            this.touched();
            this.renderSlot();
          },
        }, "Để trống góc này"),
      ),
    );
  }

  /** Chọn góc đầu tiên có lỗi (khi Lưu bị từ chối) để người dùng thấy ngay. */
  selectDir(i: number) {
    this.draft.selected = i;
    this.preview.update(this.draft.profile, i);
    this.renderSlot();
  }
}
