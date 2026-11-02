// Điều khiển tự vẽ thay cho bản gốc của webview (select, datalist, checkbox),
// để mọi danh sách thả xuống và ô tích cùng một kiểu với giao diện app.
import { h, icon, nextId } from "./dom";

// ---------- Khung nổi dùng chung ----------

export interface Popover {
  el: HTMLDivElement;
  close(): void;
}

/**
 * Mở một khung nổi bám theo `anchor`. Nằm trong hộp thoại thì gắn vào chính hộp
 * thoại (phần ngoài hộp thoại modal không bấm được). Tự đóng khi bấm ra ngoài,
 * cuộn trang hoặc đổi kích thước cửa sổ.
 */
export function openPopover(anchor: HTMLElement, cls: string, onClose: () => void): Popover {
  const host = anchor.closest("dialog") ?? document.body;
  const el = h("div", { class: `pop ${cls}` });
  host.append(el);
  let closed = false;

  const close = () => {
    if (closed) return;
    closed = true;
    el.remove();
    document.removeEventListener("pointerdown", onDown, true);
    document.removeEventListener("scroll", onScroll, true);
    window.removeEventListener("resize", close);
    window.removeEventListener("blur", close);
    onClose();
  };
  const onDown = (e: Event) => {
    if (!el.contains(e.target as Node) && !anchor.contains(e.target as Node)) close();
  };
  const onScroll = (e: Event) => {
    if (!el.contains(e.target as Node)) close();
  };
  setTimeout(() => {
    if (closed) return;
    document.addEventListener("pointerdown", onDown, true);
    document.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", close);
    window.addEventListener("blur", close);
  });
  return { el, close };
}

/** Đặt khung nổi ngay dưới (hoặc trên, nếu thiếu chỗ) phần tử neo. */
export function place(pop: HTMLElement, anchor: HTMLElement, minWidth = 0) {
  const r = anchor.getBoundingClientRect();
  const width = Math.max(r.width, minWidth);
  pop.style.minWidth = `${width}px`;
  pop.style.left = `${Math.max(8, Math.min(r.left, window.innerWidth - pop.offsetWidth - 8))}px`;
  const below = r.bottom + 6;
  const fitsBelow = below + pop.offsetHeight <= window.innerHeight - 8;
  pop.style.top = `${fitsBelow ? below : Math.max(8, r.top - pop.offsetHeight - 6)}px`;
}

// ---------- Danh sách thả xuống ----------

export interface DropdownOptions<T extends string> {
  options: ReadonlyArray<readonly [T, string]>;
  /** `null` = chưa chọn, hiện `placeholder`. */
  value: T | null;
  placeholder?: string;
  /** Nhãn cho trình đọc màn hình. */
  label: string;
  onChange: (value: T) => void;
  /** Chọn xong thì quay về `placeholder` (dùng như một menu lệnh). */
  resetAfterPick?: boolean;
  id?: string;
  className?: string;
}

export function dropdown<T extends string>(o: DropdownOptions<T>): HTMLButtonElement {
  let value = o.value;
  const text = h("span", { class: "dd-text" });
  const btn = h(
    "button",
    { type: "button", class: `dd${o.className ? ` ${o.className}` : ""}`, "aria-haspopup": "listbox", "aria-expanded": "false", "aria-label": o.label, id: o.id ?? null },
    text,
    icon("chevronDown", 16, "ico dd-chev"),
  );

  const draw = () => {
    const current = o.options.find(([v]) => v === value);
    text.textContent = current ? current[1] : (o.placeholder ?? "Chọn…");
    btn.classList.toggle("placeholder", !current);
  };
  draw();

  let pop: Popover | null = null;
  let active = 0;
  let items: HTMLDivElement[] = [];

  const setActive = (i: number) => {
    active = Math.max(0, Math.min(items.length - 1, i));
    items.forEach((it, j) => it.classList.toggle("active", j === active));
    items[active]?.scrollIntoView({ block: "nearest" });
    if (items[active]) btn.setAttribute("aria-activedescendant", items[active].id);
  };

  const pick = (i: number) => {
    const option = o.options[i];
    if (!option) return;
    value = o.resetAfterPick ? null : option[0];
    draw();
    pop?.close();
    o.onChange(option[0]);
  };

  const open = () => {
    if (pop) return;
    btn.setAttribute("aria-expanded", "true");
    pop = openPopover(btn, "dd-pop", () => {
      pop = null;
      btn.setAttribute("aria-expanded", "false");
      btn.removeAttribute("aria-activedescendant");
    });
    const listId = nextId("lb");
    pop.el.setAttribute("role", "listbox");
    pop.el.id = listId;
    btn.setAttribute("aria-controls", listId);
    items = o.options.map(([v, label], i) => {
      const it = h("div", { class: "dd-opt", role: "option", id: nextId("opt"), "aria-selected": String(v === value) }, h("span", null, label), v === value ? icon("check", 14, "ico dd-tick") : null);
      it.addEventListener("pointerdown", (e) => e.preventDefault()); // giữ focus ở nút
      it.addEventListener("click", () => pick(i));
      it.addEventListener("pointermove", () => setActive(i));
      return it;
    });
    pop.el.append(...items);
    place(pop.el, btn);
    setActive(Math.max(0, o.options.findIndex(([v]) => v === value)));
  };

  btn.addEventListener("click", () => (pop ? pop.close() : open()));
  btn.addEventListener("keydown", (e) => {
    if (!pop) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
        e.preventDefault();
        open();
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setActive(active + 1);
        break;
      case "ArrowUp":
        e.preventDefault();
        setActive(active - 1);
        break;
      case "Home":
        e.preventDefault();
        setActive(0);
        break;
      case "End":
        e.preventDefault();
        setActive(items.length - 1);
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        pick(active);
        break;
      case "Escape":
        e.preventDefault();
        e.stopPropagation();
        pop.close();
        break;
      case "Tab":
        pop.close();
        break;
      default:
        // Gõ chữ cái để nhảy tới mục bắt đầu bằng chữ đó.
        if (e.key.length === 1) {
          const k = e.key.toLowerCase();
          const start = (active + 1) % items.length;
          for (let n = 0; n < items.length; n++) {
            const i = (start + n) % items.length;
            if (o.options[i][1].toLowerCase().startsWith(k)) {
              setActive(i);
              break;
            }
          }
        }
    }
  });
  btn.addEventListener("blur", () => pop?.close());
  return btn;
}

// ---------- Ô nhập có gợi ý ----------

export interface Suggestion {
  value: string;
  note: string;
}

/**
 * Gắn danh sách gợi ý vào một ô nhập. `onPick` được gọi khi chọn một gợi ý;
 * Enter khi không có gợi ý nào đang sáng thì để ô nhập tự xử lý.
 */
export function attachSuggestions(input: HTMLInputElement, load: () => Promise<Suggestion[]>, onPick: (value: string) => void) {
  let all: Suggestion[] = [];
  let shown: Suggestion[] = [];
  let pop: Popover | null = null;
  let active = -1;
  let items: HTMLDivElement[] = [];
  load().then((list) => {
    all = list;
    if (document.activeElement === input) refresh();
  });

  const setActive = (i: number) => {
    active = i;
    items.forEach((it, j) => it.classList.toggle("active", j === i));
    items[i]?.scrollIntoView({ block: "nearest" });
  };

  const refresh = () => {
    const q = input.value.trim().toLowerCase();
    shown = all.filter((s) => !q || s.value.toLowerCase().includes(q) || s.note.toLowerCase().includes(q)).slice(0, 8);
    if (!shown.length) {
      pop?.close();
      return;
    }
    if (!pop) {
      pop = openPopover(input, "dd-pop suggest-pop", () => {
        pop = null;
        input.setAttribute("aria-expanded", "false");
      });
      pop.el.setAttribute("role", "listbox");
      input.setAttribute("aria-expanded", "true");
    }
    items = shown.map((s, i) => {
      const it = h("div", { class: "dd-opt suggest", role: "option" }, h("span", { class: "mono" }, s.value), h("span", { class: "suggest-note" }, s.note));
      it.addEventListener("pointerdown", (e) => e.preventDefault());
      it.addEventListener("click", () => choose(i));
      it.addEventListener("pointermove", () => setActive(i));
      return it;
    });
    pop.el.replaceChildren(...items);
    place(pop.el, input, 300);
    setActive(-1);
  };

  const choose = (i: number) => {
    const s = shown[i];
    if (!s) return;
    pop?.close();
    input.value = s.value;
    onPick(s.value);
  };

  input.setAttribute("role", "combobox");
  input.setAttribute("aria-autocomplete", "list");
  input.setAttribute("aria-expanded", "false");
  input.addEventListener("focus", refresh);
  input.addEventListener("input", refresh);
  input.addEventListener("blur", () => pop?.close());
  input.addEventListener("keydown", (e) => {
    if (!pop) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive(Math.min(items.length - 1, active + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive(Math.max(-1, active - 1));
    } else if (e.key === "Enter" && active >= 0) {
      e.preventDefault();
      e.stopImmediatePropagation();
      choose(active);
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopImmediatePropagation();
      pop.close();
    }
  });
}

// ---------- Ô tích ----------

/** Ô tích tự vẽ kèm nhãn và dòng giải thích. */
export function checkbox(label: string, hint: string | null, checked: boolean, onChange: (v: boolean) => void): HTMLLabelElement {
  const input = h("input", { type: "checkbox", class: "cb-input", checked });
  input.addEventListener("change", () => onChange(input.checked));
  return h(
    "label",
    { class: "check-row" },
    input,
    h("span", { class: "cbox", "aria-hidden": "true" }, icon("check", 13, "ico cb-tick")),
    h("span", { class: "check-text" }, h("span", null, label), hint ? h("span", { class: "hint" }, hint) : null),
  );
}
