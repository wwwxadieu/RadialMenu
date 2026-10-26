// Hộp thoại, thông báo nhỏ và bộ chọn icon.
import { ICON_NAMES } from "../shared/icons";
import { h, icon } from "./dom";

// ---------- Thông báo ----------

let toastTimer: number | undefined;

export function toast(message: string, tone: "ok" | "error" | "info" = "ok") {
  let el = document.getElementById("toast");
  if (!el) {
    el = h("div", { id: "toast", class: "toast", role: "status", "aria-live": "polite" });
    document.body.append(el);
  }
  el.textContent = message;
  el.dataset.tone = tone;
  el.classList.add("show");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => el?.classList.remove("show"), tone === "error" ? 6000 : 3200);
}

// ---------- Hộp thoại ----------

export interface DialogButton<T> {
  label: string;
  value: T;
  kind?: "primary" | "danger" | "ghost";
}

/** Mở hộp thoại, trả về giá trị của nút được bấm (Esc hoặc bấm ra ngoài = `dismiss`). */
export function dialog<T>(title: string, body: Node | string, buttons: DialogButton<T>[], dismiss: T): Promise<T> {
  return new Promise((resolve) => {
    const previous = document.activeElement as HTMLElement | null;
    const dlg = h("dialog", { class: "dialog", "aria-labelledby": "dlg-title" });
    const close = (value: T) => {
      dlg.close();
      dlg.remove();
      previous?.focus?.();
      resolve(value);
    };
    const actions = h("div", { class: "dialog-actions" });
    for (const b of buttons) {
      actions.append(h("button", { type: "button", class: `btn ${b.kind ?? "ghost"}`, onclick: () => close(b.value) }, b.label));
    }
    dlg.append(h("h2", { id: "dlg-title" }, title), typeof body === "string" ? h("p", null, body) : body, actions);
    dlg.addEventListener("cancel", (e) => {
      e.preventDefault();
      close(dismiss);
    });
    dlg.addEventListener("click", (e) => {
      if (e.target === dlg) close(dismiss);
    });
    document.body.append(dlg);
    dlg.showModal();
    (actions.querySelector(".primary, .danger") as HTMLElement | null)?.focus();
  });
}

export const confirmDialog = (title: string, body: string, ok: string, danger = false) =>
  dialog(title, body, [{ label: "Huỷ", value: false }, { label: ok, value: true, kind: danger ? "danger" : "primary" }], false);

// ---------- Bộ chọn icon ----------

/** Lưới icon nổi cạnh nút `anchor`; gọi `onPick` khi chọn. */
export function iconPicker(anchor: HTMLElement, current: string, onPick: (name: string) => void) {
  document.querySelector(".icon-pop")?.remove();
  const pop = h("div", { class: "icon-pop", role: "dialog", "aria-label": "Chọn icon" });
  const grid = h("div", { class: "icon-grid", role: "listbox" });
  for (const name of ICON_NAMES) {
    const btn = h(
      "button",
      { type: "button", class: "icon-cell", role: "option", "aria-selected": String(name === current), "aria-label": name, title: name },
      icon(name, 20),
    );
    btn.addEventListener("click", () => {
      onPick(name);
      done();
    });
    grid.append(btn);
  }
  pop.append(grid);
  document.body.append(pop);

  const r = anchor.getBoundingClientRect();
  const width = Math.min(320, window.innerWidth - 24);
  pop.style.width = `${width}px`;
  pop.style.left = `${Math.max(12, Math.min(r.left, window.innerWidth - width - 12))}px`;
  const below = r.bottom + 8;
  pop.style.top = `${below + pop.offsetHeight > window.innerHeight - 12 ? Math.max(12, r.top - pop.offsetHeight - 8) : below}px`;
  (grid.querySelector('[aria-selected="true"]') as HTMLElement | null ?? grid.firstElementChild as HTMLElement)?.focus();

  const onDoc = (e: Event) => {
    if (!pop.contains(e.target as Node) && e.target !== anchor) done();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      done();
      anchor.focus();
    }
  };
  function done() {
    pop.remove();
    document.removeEventListener("pointerdown", onDoc, true);
    document.removeEventListener("keydown", onKey, true);
  }
  setTimeout(() => {
    document.addEventListener("pointerdown", onDoc, true);
    document.addEventListener("keydown", onKey, true);
  });
}
