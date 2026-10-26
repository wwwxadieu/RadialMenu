// Tạo DOM gọn: h("button", { class: "btn", onclick: fn }, "Lưu").
import { ICONS } from "../shared/icons";
import { attachSuggestions, type Suggestion } from "./controls";

type Child = Node | string | number | null | undefined | false;
type AttrValue = string | number | boolean | null | undefined | ((ev: never) => void);
export type Attrs = Record<string, AttrValue>;

export function h<K extends keyof HTMLElementTagNameMap>(tag: K, attrs?: Attrs | null, ...children: Child[]): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (attrs) setAttrs(node, attrs);
  append(node, children);
  return node;
}

export function setAttrs(node: Element, attrs: Attrs) {
  for (const [key, value] of Object.entries(attrs)) {
    if (value === null || value === undefined || value === false) continue;
    if (typeof value === "function") {
      node.addEventListener(key.slice(2).toLowerCase(), value as EventListener);
    } else if (key === "value" && "value" in node) {
      (node as HTMLInputElement).value = String(value);
    } else if (key === "checked" && "checked" in node) {
      (node as HTMLInputElement).checked = Boolean(value);
    } else if (value === true) {
      node.setAttribute(key, "");
    } else {
      node.setAttribute(key, String(value));
    }
  }
}

export function append(node: Node, children: Child[]) {
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    node.appendChild(typeof c === "string" || typeof c === "number" ? document.createTextNode(String(c)) : c);
  }
}

const SVG = "http://www.w3.org/2000/svg";

export function svgEl<K extends keyof SVGElementTagNameMap>(tag: K, attrs: Record<string, string | number> = {}): SVGElementTagNameMap[K] {
  const node = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, String(v));
  return node;
}

/** Icon nét 24×24 theo tên trong ICONS. */
export function icon(name: string, size = 18, cls = "ico"): SVGSVGElement {
  const node = svgEl("svg", {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    "stroke-width": 2,
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
    "aria-hidden": "true",
    class: cls,
  });
  node.appendChild(svgEl("path", { d: ICONS[name] ?? "" }));
  return node;
}

let uid = 0;
export const nextId = (prefix = "f") => `${prefix}-${++uid}`;

/** Ô nhập kèm nhãn. */
export function field(label: string, control: HTMLElement, hint?: string): HTMLDivElement {
  const id = control.id || nextId();
  control.id = id;
  return h("div", { class: "field" }, h("label", { for: id }, label), control, hint ? h("p", { class: "hint" }, hint) : null);
}

/** Công tắc bật/tắt (checkbox có role="switch"). */
export function toggle(label: string, checked: boolean, onChange: (v: boolean) => void, hint?: string, disabled = false): HTMLLabelElement {
  const input = h("input", { type: "checkbox", role: "switch", checked, disabled });
  input.addEventListener("change", () => onChange(input.checked));
  return h(
    "label",
    { class: `switch-row${disabled ? " disabled" : ""}` },
    h("span", { class: "switch-text" }, h("span", { class: "switch-label" }, label), hint ? h("span", { class: "hint" }, hint) : null),
    input,
    h("span", { class: "switch", "aria-hidden": "true" }),
  );
}

/** Nhóm nút chọn một (radio dạng phân đoạn). */
export function segmented<T extends string>(
  name: string,
  legend: string,
  options: ReadonlyArray<readonly [T, string]>,
  value: T,
  onChange: (v: T) => void,
): HTMLFieldSetElement {
  const group = h("div", { class: "seg" });
  for (const [v, text] of options) {
    const input = h("input", { type: "radio", name, value: v, checked: v === value });
    input.addEventListener("change", () => input.checked && onChange(v));
    group.append(h("label", { class: "seg-opt" }, input, h("span", null, text)));
  }
  return h("fieldset", { class: "seg-field" }, h("legend", null, legend), group);
}

/** Chip có nút xoá. */
export function chip(text: string, onRemove?: () => void, mono = true): HTMLSpanElement {
  return h(
    "span",
    { class: `chip${mono ? " mono" : ""}` },
    text,
    onRemove ? h("button", { type: "button", class: "chip-x", "aria-label": `Bỏ ${text}`, onclick: onRemove }, icon("x", 12)) : null,
  );
}

/**
 * Nút "+ Thêm…" trong hàng chip: bấm thì thành ô nhập (có gợi ý nếu truyền
 * `suggest`). Enter, chọn một gợi ý hoặc rời ô thì thêm; Esc thì thôi.
 */
export function chipAdder(
  label: string,
  placeholder: string,
  onAdd: (value: string) => void,
  suggest?: () => Promise<Suggestion[]>,
): HTMLButtonElement {
  const btn = h("button", { type: "button", class: "chip add" }, label);
  btn.addEventListener("click", () => {
    const input = h("input", { type: "text", class: "chip-input mono", placeholder, spellcheck: "false", autocomplete: "off", "aria-label": label.replace(/^\+\s*/, "") });
    const wrap = h("span", { class: "chip-edit" }, input);
    btn.replaceWith(wrap);
    let done = false;
    const finish = (commit: boolean) => {
      if (done) return;
      done = true;
      const value = input.value.trim();
      wrap.replaceWith(btn);
      if (commit && value) onAdd(value);
    };
    // Gắn gợi ý trước để Enter trên một gợi ý được xử lý trước.
    if (suggest) attachSuggestions(input, suggest, () => finish(true));
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        finish(true);
      } else if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        finish(false);
        btn.focus();
      }
    });
    input.addEventListener("blur", () => setTimeout(() => finish(true), 150));
    input.focus();
  });
  return btn;
}
