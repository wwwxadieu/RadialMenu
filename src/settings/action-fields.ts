// Ô nhập cho từng loại hành động. Các hàm ở đây sửa thẳng vào object hành động
// của bản nháp rồi báo `host.changed()`; đổi cấu trúc (đổi loại, thêm bước) thì
// báo `host.restructure()` để vẽ lại.
import { asModifier, joinChord, keyFromCode, MODIFIERS, prettyKey, SPECIAL_KEYS, splitChord } from "../shared/keys";
import type { Action, ActionType } from "../shared/types";
import { attachSuggestions, dropdown } from "./controls";
import { field, h, icon, nextId, segmented } from "./dom";

export interface ActionHost {
  changed(): void;
  restructure(): void;
}

export const TOP_TYPES: ReadonlyArray<readonly [ActionType, string]> = [
  ["hotkey", "Phím tắt"],
  ["command", "Chạy lệnh"],
  ["url", "Mở URL"],
  ["text", "Gõ văn bản"],
  ["sequence", "Chuỗi bước"],
  ["settings", "Mở cài đặt"],
  ["openWith", "Mở bằng"],
  ["submenu", "Menu con"],
];

/** Loại hành động cho một góc trong menu con: không lồng thêm menu con. */
export const SUB_TYPES = TOP_TYPES.filter(([t]) => t !== "submenu");

const STEP_TYPES: ReadonlyArray<readonly [ActionType, string]> = [
  ["hotkey", "Phím tắt"],
  ["text", "Gõ văn bản"],
  ["wait", "Chờ"],
  ["command", "Chạy lệnh"],
  ["url", "Mở URL"],
];

/** Lệnh Dán (Ctrl+V, Shift+Insert): khớp `Action::is_paste` bên Rust. */
export function isPaste(a: Action): boolean {
  if (a.type !== "hotkey") return false;
  const names = a.keys
    .map((k) => k.trim().toLowerCase())
    .map((k) => (k === "control" ? "ctrl" : k === "ins" ? "insert" : k))
    .sort()
    .join("+");
  return names === "ctrl+v" || names === "insert+shift";
}

export function defaultAction(type: ActionType): Action {
  switch (type) {
    case "hotkey":
      return { type, keys: [] };
    case "command":
      return { type, cmd: "" };
    case "url":
      return { type, url: "" };
    case "text":
      return { type, text: "" };
    case "sequence":
      return { type, steps: [{ type: "hotkey", keys: [] }] };
    case "wait":
      return { type, ms: 150 };
    case "settings":
      return { type };
    case "openWith":
      return { type, app: "" };
    case "submenu":
      return { type, slots: [] };
  }
}

/** Chương trình hay dùng để mở file, gợi ý khi gõ. */
const OPEN_WITH_APPS = [
  { value: "notepad.exe", note: "Notepad" },
  { value: "code", note: "Visual Studio Code" },
  { value: "mspaint.exe", note: "Paint" },
  { value: "msedge.exe", note: "Microsoft Edge" },
  { value: "chrome.exe", note: "Google Chrome" },
  { value: "winword.exe", note: "Microsoft Word" },
  { value: "excel.exe", note: "Microsoft Excel" },
  { value: "photoshop.exe", note: "Adobe Photoshop" },
];

function openWithFields(action: Extract<Action, { type: "openWith" }>, host: ActionHost): HTMLElement {
  const mode = action.app === undefined ? "dialog" : "app";
  const choose = segmented(groupName(), "Mở file bằng", [["app", "Chương trình chỉ định"], ["dialog", "Hỏi mỗi lần (hộp thoại Windows)"]] as const, mode, (m) => {
    if (m === "dialog") delete action.app;
    else action.app = "";
    host.changed();
    host.restructure();
  });
  const box = h("div", { class: "stack" }, choose);
  if (mode === "app") {
    const app = input(action.app ?? "", (v) => {
      action.app = v;
      host.changed();
    }, { class: "input mono", placeholder: "notepad.exe, code, C:\\Tools\\app.exe" });
    attachSuggestions(app, async () => OPEN_WITH_APPS, (v) => {
      action.app = v;
      host.changed();
    });
    box.append(field("Chương trình", app, "Tên chương trình trong PATH, tên đã đăng ký với Windows (như msedge.exe) hoặc đường dẫn đầy đủ tới file .exe."));
  }
  box.append(
    h(
      "p",
      { class: "hint" },
      "Mở file hoặc thư mục đang chọn trong File Explorer hay trên Desktop. Nên bật “Click vào chỗ vừa nhấn trước khi chạy” để chọn đúng file dưới con trỏ; tắt nếu muốn mở cùng lúc nhiều file đã chọn sẵn.",
    ),
  );
  return box;
}

// ---------- Tách / ghép tham số dòng lệnh ----------

export function splitArgs(text: string): string[] {
  const out: string[] = [];
  const re = /"([^"]*)"|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) out.push(m[1] ?? m[2]);
  return out;
}

export const joinArgs = (args: string[] = []) => args.map((a) => (/\s/.test(a) || a === "" ? `"${a}"` : a)).join(" ");

// ---------- Phím tắt ----------

function hotkeyFields(action: Extract<Action, { type: "hotkey" }>, host: ActionHost, compact: boolean): HTMLElement {
  const chord = splitChord(action.keys);
  const wrap = h("div", { class: "hotkey" });
  const modButtons = new Map<string, HTMLButtonElement>();

  const commit = () => {
    action.keys = joinChord(chord);
    render();
    host.changed();
  };

  const mods = h("div", { class: "mods", role: "group", "aria-label": "Phím bổ trợ" });
  for (const m of MODIFIERS) {
    const btn = h("button", { type: "button", class: "mod" }, m);
    btn.addEventListener("click", () => {
      if (chord.mods.has(m)) chord.mods.delete(m);
      else chord.mods.add(m);
      commit();
    });
    modButtons.set(m, btn);
    mods.append(btn);
  }

  // Ô ghi phím: bấm vào rồi nhấn phím cần gửi.
  const keycap = h("button", { type: "button", class: "keycap capture", "aria-describedby": "" });
  let recording = false;
  const stop = () => {
    recording = false;
    keycap.classList.remove("rec");
    render();
  };
  keycap.addEventListener("click", () => {
    recording = !recording;
    keycap.classList.toggle("rec", recording);
    render();
  });
  keycap.addEventListener("blur", () => recording && stop());
  keycap.addEventListener("keydown", (e) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (asModifier(e.key === "Control" ? "ctrl" : e.key === "Meta" ? "win" : e.key)) return; // chờ phím chính
    if (e.key === "Escape" && !e.ctrlKey && !e.shiftKey && !e.altKey) {
      stop();
      return;
    }
    const key = keyFromCode(e.code);
    if (!key) return;
    chord.key = key;
    // Có giữ phím bổ trợ khi ghi thì lấy đúng tổ hợp đã bấm; không giữ thì giữ nguyên
    // các nút đã bật. Phím Win thường bị Windows giữ lại, nên lấy theo nút Win.
    if (e.ctrlKey || e.shiftKey || e.altKey || e.metaKey) {
      const held = new Set(MODIFIERS.filter((m) => (m === "Ctrl" && e.ctrlKey) || (m === "Shift" && e.shiftKey) || (m === "Alt" && e.altKey) || (m === "Win" && e.metaKey)));
      if (chord.mods.has("Win")) held.add("Win");
      chord.mods = held;
    }
    recording = false;
    keycap.classList.remove("rec");
    commit();
  });

  const special = dropdown({
    options: SPECIAL_KEYS,
    value: null,
    placeholder: "Phím khác…",
    label: "Chọn phím đặc biệt",
    resetAfterPick: true,
    className: "special",
    onChange: (key) => {
      chord.key = key;
      commit();
    },
  });

  function render() {
    for (const [m, btn] of modButtons) btn.setAttribute("aria-pressed", String(chord.mods.has(m as never)));
    keycap.textContent = recording ? "Nhấn phím…" : chord.key ? prettyKey(chord.key) : "Ghi phím";
    keycap.classList.toggle("unset", !chord.key && !recording);
    keycap.setAttribute("aria-label", recording ? "Đang ghi, nhấn phím cần gửi (Esc để thôi)" : `Phím chính: ${chord.key ? prettyKey(chord.key) : "chưa có"}. Bấm để ghi lại`);
  }
  render();

  wrap.append(h("div", { class: "hotkey-row" }, mods, h("span", { class: "plus", "aria-hidden": "true" }, "+"), keycap, special));
  if (!compact) {
    wrap.append(h("p", { class: "hint" }, "Bấm ô phím rồi nhấn phím cần gửi. Tổ hợp có phím Win: bật nút Win trước rồi ghi phím còn lại."));
  }
  return wrap;
}

// ---------- Các loại còn lại ----------

function input(value: string, onInput: (v: string) => void, attrs: Record<string, string> = {}): HTMLInputElement {
  const el = h("input", { type: "text", class: "input", value, spellcheck: "false", ...attrs });
  el.addEventListener("input", () => onInput(el.value));
  return el;
}

function commandFields(action: Extract<Action, { type: "command" }>, host: ActionHost, compact: boolean): HTMLElement {
  const cmd = input(action.cmd, (v) => {
    action.cmd = v;
    host.changed();
  }, { class: "input mono", placeholder: "wt, notepad, C:\\Tools\\app.exe" });
  const args = input(joinArgs(action.args), (v) => {
    const list = splitArgs(v);
    if (list.length) action.args = list;
    else delete action.args;
    host.changed();
  }, { class: "input mono", placeholder: "-d %DESKTOP%" });
  const box = h("div", { class: compact ? "grid-2" : "stack" }, field("Chương trình", cmd), field("Tham số", args));
  if (!compact) {
    const cwd = input(action.cwd ?? "", (v) => {
      if (v.trim()) action.cwd = v;
      else delete action.cwd;
      host.changed();
    }, { class: "input mono", placeholder: "Để trống = thư mục của bạn" });
    box.append(field("Chạy trong thư mục", cwd, "Dùng được %DESKTOP%, %DOCUMENTS%, %DOWNLOADS%, %USERPROFILE% và biến môi trường khác."));
  }
  return box;
}

function urlFields(action: Extract<Action, { type: "url" }>, host: ActionHost): HTMLElement {
  return field(
    "Địa chỉ hoặc đường dẫn",
    input(action.url, (v) => {
      action.url = v;
      host.changed();
    }, { placeholder: "https://… hoặc %DOWNLOADS%" }),
  );
}

function textFields(action: Extract<Action, { type: "text" }>, host: ActionHost, compact: boolean): HTMLElement {
  const area = h("textarea", { class: "input", rows: compact ? 2 : 3, placeholder: "Đoạn chữ hay dùng", spellcheck: "false" });
  area.value = action.text;
  area.addEventListener("input", () => {
    action.text = area.value;
    host.changed();
  });
  return field("Văn bản sẽ gõ", area, compact ? undefined : "Gõ đúng từng ký tự, kể cả tiếng Việt có dấu và xuống dòng.");
}

function waitFields(action: Extract<Action, { type: "wait" }>, host: ActionHost): HTMLElement {
  const el = h("input", { type: "number", class: "input num", min: 0, max: 5000, step: 50, value: action.ms });
  el.addEventListener("input", () => {
    action.ms = Math.max(0, Math.min(5000, Math.round(Number(el.value) || 0)));
    host.changed();
  });
  return h("div", { class: "inline-field" }, field("Chờ", el), h("span", { class: "unit" }, "mili giây"));
}

function sequenceFields(action: Extract<Action, { type: "sequence" }>, host: ActionHost): HTMLElement {
  const list = h("ol", { class: "steps" });
  action.steps.forEach((step, i) => {
    const typeSel = dropdown<ActionType>({
      options: STEP_TYPES,
      value: step.type,
      label: `Loại bước ${i + 1}`,
      className: "step-type",
      onChange: (t) => {
        if (t === step.type) return;
        action.steps[i] = defaultAction(t);
        host.changed();
        host.restructure();
      },
    });
    const move = (delta: number) => {
      const j = i + delta;
      if (j < 0 || j >= action.steps.length) return;
      [action.steps[i], action.steps[j]] = [action.steps[j], action.steps[i]];
      host.changed();
      host.restructure();
    };
    const tools = h(
      "div",
      { class: "step-tools" },
      h("button", { type: "button", class: "icon-btn sm", "aria-label": `Đưa bước ${i + 1} lên`, disabled: i === 0, onclick: () => move(-1) }, icon("arrowLeft", 14, "ico rot90")),
      h("button", { type: "button", class: "icon-btn sm", "aria-label": `Đưa bước ${i + 1} xuống`, disabled: i === action.steps.length - 1, onclick: () => move(1) }, icon("arrowRight", 14, "ico rot90")),
      h("button", {
        type: "button",
        class: "icon-btn sm",
        "aria-label": `Xoá bước ${i + 1}`,
        disabled: action.steps.length === 1,
        onclick: () => {
          action.steps.splice(i, 1);
          host.changed();
          host.restructure();
        },
      }, icon("x", 14)),
    );
    list.append(h("li", { class: "step" }, h("div", { class: "step-head" }, h("span", { class: "step-no" }, String(i + 1)), typeSel, tools), actionFields(step, host, true)));
  });
  const add = h("button", {
    type: "button",
    class: "btn ghost dashed",
    onclick: () => {
      action.steps.push(defaultAction("hotkey"));
      host.changed();
      host.restructure();
    },
  }, icon("plus", 14), "Thêm bước");
  return h("div", { class: "sequence" }, list, add, h("p", { class: "hint" }, "Các bước chạy lần lượt từ trên xuống. Thêm bước Chờ nếu app cần thời gian phản hồi."));
}

export function actionFields(action: Action, host: ActionHost, compact = false): HTMLElement {
  switch (action.type) {
    case "hotkey":
      return hotkeyFields(action, host, compact);
    case "command":
      return commandFields(action, host, compact);
    case "url":
      return urlFields(action, host);
    case "text":
      return textFields(action, host, compact);
    case "wait":
      return waitFields(action, host);
    case "sequence":
      return compact ? h("p", { class: "hint" }, "Không lồng chuỗi bước vào nhau.") : sequenceFields(action, host);
    case "settings":
      return h("p", { class: "hint" }, "Mở cửa sổ cài đặt này của Radial Menu. Không cần nhập gì thêm.");
    case "openWith":
      return openWithFields(action, host);
    case "submenu":
      // Trình sửa profile tự vẽ phần này (cần chuyển sang sửa menu con).
      return h("p", { class: "hint" }, "Menu con: thêm tối đa 8 lệnh nữa ở góc này.");
  }
}

export const typeLabel = (t: ActionType) => [...TOP_TYPES, ...STEP_TYPES].find(([v]) => v === t)?.[1] ?? t;

// id duy nhất cho nhóm radio của mỗi lần vẽ.
export const groupName = () => nextId("type");
