// Trang sửa một profile: tên, icon, app áp dụng, và lệnh của 8 góc.
// Mọi thay đổi nằm trong bản nháp (`Draft`) tới khi bấm Lưu.
import { DIRS } from "../shared/geometry";
import type { AppState, DirId, Profile, Slot } from "../shared/types";
import { actionFields, defaultAction, groupName, isPaste, SUB_TYPES, TOP_TYPES, type ActionHost } from "./action-fields";
import { checkbox } from "./controls";
import { chip, chipAdder, h, icon, segmented, toggle } from "./dom";
import { libraryPanel, type Preset } from "./library";
import { iconPicker, toast } from "./overlay";
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
const findIn = (slots: Slot[], i: number) => slots.find((s) => s.dir === DIRS[i].id) ?? null;
const byDir = (a: Slot, b: Slot) => DIRS.findIndex((x) => x.id === a.dir) - DIRS.findIndex((x) => x.id === b.dir);

/** Bảng bên phải: sửa góc đang chọn, hoặc thư viện lệnh để kéo thả. Giữ qua các lần vẽ lại. */
let panel: "slot" | "library" = "slot";

export class ProfileEditor {
  readonly el: HTMLElement;
  private preview: RadialPreview;
  private slotBox = h("section", { class: "card slot-card" });
  private slotBody = h("div", { class: "slot-body", role: "tabpanel" });
  private tabs = h("div", { class: "panel-tabs", role: "tablist", "aria-label": "Bảng bên phải" });
  private note = h("p", { class: "card-note" });
  private library: ReturnType<typeof libraryPanel> | null = null;
  private headBox = h("section", { class: "card head-card", "aria-label": "Thông tin profile" });
  /** Hiện ô thêm lớp cửa sổ (tuỳ chọn nâng cao). */
  private advanced = false;
  /** Đang sửa menu con của góc này (`null` = vòng chính). */
  private sub: number | null = null;
  /** Góc đang chọn trong menu con. */
  private subSelected = 0;

  constructor(private draft: Draft, private host: EditorHost) {
    this.preview = new RadialPreview({
      interactive: true,
      label: "Xem trước menu, chọn góc cần sửa",
      onPick: (i) => {
        this.selected = i;
        this.refreshPreview();
        // Bấm vào một góc = muốn sửa góc đó.
        panel = "slot";
        this.renderPanel();
      },
    });
    const previewCard = h("section", { class: "card preview-card", "aria-label": "Xem trước menu" }, this.note, this.preview.el);
    this.slotBox.append(this.tabs, this.slotBody);
    this.el = h("div", { class: "page profile-page" }, this.headBox, h("div", { class: "editor-grid" }, previewCard, this.slotBox));
    this.renderHead();
    this.renderPanel();
    this.refreshPreview();
  }

  // ---------- Vòng đang sửa: vòng chính hoặc một menu con ----------

  /** Góc đang mở menu con (nếu góc đó vẫn là menu con). */
  private parentSlot(): (Slot & { action: { type: "submenu"; slots: Slot[] } }) | null {
    if (this.sub === null) return null;
    const parent = slotAt(this.draft.profile, this.sub);
    if (parent && parent.action.type === "submenu") return parent as Slot & { action: { type: "submenu"; slots: Slot[] } };
    this.sub = null;
    return null;
  }

  /** Mảng góc của vòng đang sửa (sửa thẳng vào bản nháp). */
  private ring(): Slot[] {
    return this.parentSlot()?.action.slots ?? this.draft.profile.slots;
  }

  private get selected(): number {
    return this.parentSlot() ? this.subSelected : this.draft.selected;
  }

  private set selected(i: number) {
    if (this.parentSlot()) this.subSelected = i;
    else this.draft.selected = i;
  }

  /** Vào sửa menu con của góc `parent` của vòng chính. */
  private enterSub(parent: number, child = 0) {
    this.sub = parent;
    this.subSelected = child;
    panel = "slot";
    this.renderPanel();
    this.refreshPreview();
  }

  private leaveSub() {
    if (this.sub === null) return;
    const parent = this.sub;
    this.sub = null;
    this.draft.selected = parent;
    // Quay lại thì hiện luôn góc vừa sửa (tóm tắt menu con của nó).
    panel = "slot";
    this.renderPanel();
    this.refreshPreview();
  }

  /** Vẽ hình menu theo vòng đang sửa. */
  private refreshPreview() {
    const parent = this.parentSlot();
    if (!parent) {
      this.preview.update(this.draft.profile, this.draft.selected);
      return;
    }
    const view: Profile = {
      ...this.draft.profile,
      name: parent.label || "Menu con",
      icon: parent.icon || "grid",
      slots: parent.action.slots,
    };
    this.preview.update(view, this.subSelected, "menu con");
  }

  /** Dòng trên hình menu: lời nhắc, hoặc đường dẫn quay lại khi đang ở menu con. */
  private renderNote() {
    const parent = this.parentSlot();
    if (!parent) {
      this.note.className = "card-note";
      this.note.textContent = panel === "library" ? "Thả lệnh vào một góc để gán cho góc đó" : "Bấm vào một góc để sửa lệnh của góc đó";
      return;
    }
    this.note.className = "card-note crumb";
    this.note.replaceChildren(
      h("button", { type: "button", class: "crumb-back", onclick: () => this.leaveSub() }, icon("arrowLeft", 15), this.draft.profile.name || "Profile"),
      h("span", { class: "crumb-sep", "aria-hidden": "true" }, "/"),
      h("span", { class: "crumb-here" }, icon(parent.icon || "grid", 15), `Menu con “${parent.label || "chưa đặt tên"}”`),
    );
  }

  // ---------- Bảng bên phải: Sửa góc / Thư viện lệnh ----------

  private renderPanel() {
    const tab = (id: typeof panel, ico: string, label: string) =>
      h(
        "button",
        {
          type: "button",
          role: "tab",
          class: "panel-tab",
          "aria-selected": String(panel === id),
          tabindex: panel === id ? "0" : "-1",
          onclick: () => {
            if (panel === id) return;
            panel = id;
            this.renderPanel();
          },
        },
        icon(ico, 16),
        label,
      );
    this.tabs.replaceChildren(tab("slot", "pencil", "Sửa góc"), tab("library", "grid", "Thư viện lệnh"));
    // Mũi tên trái/phải chuyển giữa hai thẻ.
    this.tabs.onkeydown = (e) => {
      if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
      e.preventDefault();
      panel = panel === "slot" ? "library" : "slot";
      this.renderPanel();
      (this.tabs.querySelector('[aria-selected="true"]') as HTMLElement | null)?.focus();
    };
    this.renderNote();
    if (panel === "library") {
      this.library = libraryPanel({
        selected: () => this.selected,
        dirName: (i) => (this.parentSlot() ? `${DIRS[i].name} của menu con` : DIRS[i].name),
        dirAt: (x, y) => this.preview.dirAt(x, y),
        hover: (i) => this.preview.setDrop(i),
        assign: (i, preset) => this.assign(i, preset),
      });
      this.slotBody.replaceChildren(this.library.el);
    } else {
      this.library = null;
      this.renderSlot();
    }
  }

  /** Gán một lệnh trong thư viện vào góc `i` (thay lệnh cũ nếu có). */
  private assign(i: number, preset: Preset) {
    const slots = this.ring();
    const dir = DIRS[i].id as DirId;
    const slot: Slot = { dir, label: preset.label, icon: preset.icon, action: structuredClone(preset.action) };
    if (preset.clickFirst) slot.clickFirst = true;
    const at = slots.findIndex((s) => s.dir === dir);
    const replaced = at >= 0 ? slots[at].label : null;
    if (at >= 0 && slots[at].action.type === "submenu" && !this.parentSlot()) {
      toast(`Góc ${DIRS[i].name} đang là menu con. Mở menu con rồi thả lệnh vào trong, hoặc đổi loại của góc trước.`, "info");
      return;
    }
    if (preset.action.type === "submenu" && this.parentSlot()) {
      toast("Không đặt menu con bên trong menu con. Quay lại vòng chính rồi thả vào một góc.", "info");
      return;
    }
    if (at >= 0) slots[at] = slot;
    else {
      slots.push(slot);
      slots.sort(byDir);
    }
    this.selected = i;
    this.touched();
    this.preview.flash(i);
    this.library?.refresh();
    toast(
      replaced && replaced !== preset.label
        ? `Góc ${DIRS[i].name}: “${replaced}” → “${preset.label}”. Bấm Lưu để giữ.`
        : `Đã gán “${preset.label}” vào góc ${DIRS[i].name}. Bấm Lưu để giữ.`,
      "info",
    );
  }

  /** Bản nháp đổi: vẽ lại phần xem trước và báo ra ngoài. */
  private touched() {
    this.refreshPreview();
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
    const slots = this.ring();
    const parent = this.parentSlot();
    const i = this.selected;
    const d = DIRS[i];
    const slot = findIn(slots, i);
    const title = h("h2", { id: "slot-title" }, slot ? slot.label || "Chưa đặt tên" : "Góc trống");
    const where = parent ? `MENU CON “${(parent.label || "CHƯA ĐẶT TÊN").toUpperCase()}” · ` : "";
    const head = h("div", { class: "slot-head" }, h("span", { class: "eyebrow" }, `${where}GÓC ${d.name.toUpperCase()} ${d.arrow}`), title);

    if (!slot) {
      this.slotBody.replaceChildren(
        head,
        h("p", { class: "muted" }, "Nhả chuột ở góc này sẽ không làm gì."),
        h("button", {
          type: "button",
          class: "btn primary",
          onclick: () => {
            slots.push({ dir: d.id as DirId, label: "", icon: "star", action: defaultAction("hotkey") });
            slots.sort(byDir);
            this.touched();
            this.renderSlot();
            (this.slotBody.querySelector(".label-input") as HTMLInputElement | null)?.focus();
          },
        }, icon("plus", 16), "Tự đặt lệnh cho góc này"),
        h("button", {
          type: "button",
          class: "btn ghost",
          onclick: () => {
            panel = "library";
            this.renderPanel();
          },
        }, icon("grid", 16), "Chọn từ thư viện lệnh"),
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

    // Trong menu con không lồng thêm menu con.
    const types = segmented(groupName(), "Loại hành động", parent ? SUB_TYPES : TOP_TYPES, slot.action.type, (t) => {
      if (slot.action.type === "submenu" && slot.action.slots.length && t !== "submenu") {
        toast(`Đã bỏ ${slot.action.slots.length} lệnh của menu con. Bấm Huỷ thay đổi nếu đổi nhầm.`, "info");
      }
      slot.action = defaultAction(t);
      // Mở bằng cần chọn đúng file dưới con trỏ: bật sẵn click trước.
      if (t === "openWith") slot.clickFirst = true;
      this.touched();
      this.renderSlot();
    });

    if (slot.action.type === "submenu") {
      this.slotBody.replaceChildren(
        head,
        h("div", { class: "field" }, h("label", { for: "slot-label" }, "Tên hiển thị"), h("div", { class: "label-row" }, iconBtn, label)),
        types,
        this.submenuSummary(slot.action.slots, i),
        h("div", { class: "slot-foot" }, this.clearButton(slots, slot)),
      );
      return;
    }

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

    this.slotBody.replaceChildren(
      head,
      h("div", { class: "field" }, h("label", { for: "slot-label" }, "Tên hiển thị"), h("div", { class: "label-row" }, iconBtn, label)),
      types,
      actionFields(slot.action, host),
      h("div", { class: "checks" }, click, clip),
      h("div", { class: "slot-foot" }, this.clearButton(slots, slot)),
    );
  }

  private clearButton(slots: Slot[], slot: Slot): HTMLButtonElement {
    return h("button", {
      type: "button",
      class: "btn ghost",
      onclick: () => {
        const k = slots.indexOf(slot);
        if (k >= 0) slots.splice(k, 1);
        this.touched();
        this.renderSlot();
      },
    }, "Để trống góc này");
  }

  /** Góc là menu con: tóm tắt 8 góc của nó và nút vào sửa. */
  private submenuSummary(children: Slot[], parent: number): HTMLElement {
    const grid = h(
      "div",
      { class: "sub-grid" },
      ...DIRS.map((dir, k) => {
        const c = findIn(children, k);
        return h(
          "button",
          { type: "button", class: `sub-cell${c ? "" : " empty"}`, onclick: () => this.enterSub(parent, k), title: c ? `Sửa ${c.label}` : `Thêm lệnh vào góc ${dir.name}` },
          h("span", { class: "sub-arrow", "aria-hidden": "true" }, dir.arrow),
          c ? icon(c.icon || "star", 15) : null,
          h("span", { class: "sub-label" }, c ? c.label || "Chưa đặt tên" : "Trống"),
        );
      }),
    );
    return h(
      "div",
      { class: "submenu-box" },
      h(
        "p",
        { class: "hint" },
        children.length
          ? `Menu con có ${children.length} lệnh. Khi dùng: kéo vụt qua mép menu ở góc này, hoặc nhả chuột ở góc này rồi di chuột và bấm vào lệnh cần chạy.`
          : "Menu con chưa có lệnh nào. Mở menu con để thêm lệnh, tối đa 8 lệnh.",
      ),
      grid,
      h("button", { type: "button", class: "btn primary", onclick: () => this.enterSub(parent, children.length ? DIRS.findIndex((x) => x.id === children[0].dir) : 0) }, icon("arrowRight", 16), "Mở menu con để sửa"),
    );
  }

  /** Chọn góc đầu tiên có lỗi (khi Lưu bị từ chối) để người dùng thấy ngay. */
  selectDir(i: number, child: number | null = null) {
    if (child !== null && slotAt(this.draft.profile, i)?.action.type === "submenu") {
      this.enterSub(i, child);
      return;
    }
    this.sub = null;
    this.draft.selected = i;
    panel = "slot";
    this.renderPanel();
    this.refreshPreview();
  }
}
