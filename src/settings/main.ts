// Cửa sổ cài đặt: danh sách profile + các trang chung. Thay cho cửa sổ nhật ký cũ.
import { listen } from "@tauri-apps/api/event";
import { applyAccent } from "../shared/color";
import logoSvg from "../shared/logo.svg?raw";
import type { AppInfo, AppState, GestureEvent, Profile, Settings } from "../shared/types";
import { api, errorText } from "./api";
import { dropdown } from "./controls";
import { h, icon, toggle } from "./dom";
import { isDirty, makeDraft, ProfileEditor, type Draft } from "./editor";
import { dialog, toast } from "./overlay";
import { appearancePage, backupPage, behaviorPage, describeEvent, logPage, type LogEntry, type PageHost } from "./pages";
import "./style.css";

type Page =
  | { kind: "profile"; id: string }
  | { kind: "new" }
  | { kind: "behavior" }
  | { kind: "appearance" }
  | { kind: "log" }
  | { kind: "backup" };

const GENERAL: ReadonlyArray<{ kind: Exclude<Page["kind"], "profile" | "new">; label: string; icon: string }> = [
  { kind: "behavior", label: "Cử chỉ & khởi động", icon: "mouse" },
  { kind: "appearance", label: "Giao diện", icon: "sun" },
  { kind: "log", label: "Nhật ký", icon: "format" },
  { kind: "backup", label: "Sao lưu & khôi phục", icon: "save" },
];

const side = document.getElementById("side") as HTMLElement;
const main = document.getElementById("main") as HTMLElement;
const savebar = document.getElementById("savebar") as HTMLElement;

let app: AppState;
let page: Page = { kind: "profile", id: "default" };
let draft: Draft | null = null;
let editor: ProfileEditor | null = null;
let saveError: string | null = null;
let saving = false;
const logEntries: LogEntry[] = [];
let logView: ReturnType<typeof logPage> | null = null;
/** Bỏ qua lần vẽ lại do chính cửa sổ này vừa lưu. */
let suppressUntil = 0;

// ---------- Tiện ích ----------

const sleep = (ms: number) => new Promise((r) => window.setTimeout(r, ms));
const collator = new Intl.Collator("vi");

function sortedProfiles(list: Profile[]): Profile[] {
  return [...list].sort((a, b) => (a.id === "default" ? -1 : b.id === "default" ? 1 : collator.compare(a.name, b.name)));
}

function applyTheme(s: Settings) {
  document.documentElement.dataset.tone = s.tone;
  document.documentElement.dataset.motion = s.animations;
  applyAccent(document.documentElement, s.accent);
}

let appsCache: { at: number; list: AppInfo[] } | null = null;

/** Gợi ý tên chương trình: app gần đây dưới con trỏ + app đang mở cửa sổ. */
async function suggestions(): Promise<{ value: string; note: string }[]> {
  if (!appsCache || Date.now() - appsCache.at > 5000) {
    appsCache = { at: Date.now(), list: await api.listApps().catch(() => []) };
  }
  const out = new Map<string, string>();
  for (const p of app.recent) out.set(p, "vừa dùng menu ở đây");
  for (const a of appsCache.list) if (!out.has(a.process)) out.set(a.process, a.title);
  return [...out].map(([value, note]) => ({ value, note }));
}

// ---------- Thanh bên ----------

function navButton(label: string, ico: string, active: boolean, onClick: () => void, extra?: Node | null): HTMLButtonElement {
  return h(
    "button",
    { type: "button", class: `nav-item${active ? " active" : ""}`, "aria-current": active ? "page" : null, onclick: onClick },
    icon(ico, 18),
    h("span", { class: "nav-text" }, label),
    extra ?? null,
  );
}

function renderSide() {
  const list = h("div", { class: "nav-group", role: "list" });
  const draftId = draft && (page.kind === "profile" || page.kind === "new") ? draft.originalId : undefined;
  const dirtyDot = () => h("span", { class: "dot", title: "Có thay đổi chưa lưu", "aria-label": "chưa lưu" });

  if (page.kind === "new" && draft) {
    list.append(navButton(draft.profile.name || "Profile mới", draft.profile.icon, true, () => {}, h("span", { class: "badge" }, "mới")));
  }
  for (const p of sortedProfiles(app.profiles)) {
    const active = page.kind === "profile" && page.id === p.id;
    const name = active && draft ? draft.profile.name || p.name : p.name;
    const ico = active && draft ? draft.profile.icon : p.icon;
    const extra = active && draft && draftId === p.id && isDirty(draft) ? dirtyDot() : !p.enabled ? h("span", { class: "badge muted" }, "tắt") : null;
    list.append(navButton(name, ico, active, () => navigate({ kind: "profile", id: p.id }), extra));
  }

  const pause = toggle("Bật menu", !app.paused, async (on) => {
    // Lệnh trả về trạng thái tạm dừng mới (true = đang tạm dừng).
    app.paused = await api.setPaused(!on);
    renderSide();
  });
  pause.classList.add("side-switch");

  side.replaceChildren(
    h("div", { class: "brand" }, brandMark(), h("span", { class: "brand-name" }, "Radial Menu")),
    h(
      "nav",
      { class: "nav", "aria-label": "Điều hướng" },
      h("div", { class: "nav-title" }, "PROFILE"),
      list,
      h("button", { type: "button", class: "nav-add", onclick: () => addAppDialog() }, icon("plus", 14), "Thêm ứng dụng"),
      h("div", { class: "nav-title" }, "CHUNG"),
      h("div", { class: "nav-group" }, ...GENERAL.map((g) => navButton(g.label, g.icon, page.kind === g.kind, () => navigate({ kind: g.kind })))),
    ),
    h("div", { class: "side-foot" }, pause, h("span", { class: "version" }, `Phiên bản ${app.version}`)),
  );
}

/** Logo: đúng file SVG dùng để sinh icon của app (scripts/make-icons.cjs). */
function brandMark(): HTMLElement {
  const box = h("span", { class: "brand-mark", "aria-hidden": "true" });
  box.innerHTML = logoSvg;
  return box;
}

// ---------- Thanh lưu ----------

function renderSavebar() {
  const show = Boolean(draft && (isDirty(draft) || saveError));
  savebar.hidden = !show;
  if (!show || !draft) return;
  savebar.replaceChildren(
    h(
      "div",
      { class: "savebar-inner" },
      h("span", { class: `savebar-text${saveError ? " error" : ""}`, role: saveError ? "alert" : null }, saveError ?? (draft.originalId ? "Có thay đổi chưa lưu" : "Profile mới chưa lưu")),
      h("button", { type: "button", class: "btn ghost", onclick: () => revertDraft(), disabled: saving }, draft.originalId ? "Huỷ thay đổi" : "Bỏ profile này"),
      h("button", { type: "button", class: "btn primary", onclick: () => saveDraft(), disabled: saving }, saving ? "Đang lưu…" : "Lưu", h("kbd", { class: "kbd-hint" }, "Ctrl+S")),
    ),
  );
}

async function saveDraft(): Promise<boolean> {
  if (!draft || saving) return false;
  saving = true;
  saveError = null;
  renderSavebar();
  try {
    suppressUntil = Date.now() + 1500;
    const saved = await api.saveProfile(draft.profile, draft.originalId);
    app = await api.state();
    const selected = draft.selected;
    draft = makeDraft(app.profiles.find((p) => p.id === saved.id) ?? saved);
    draft.selected = selected;
    page = { kind: "profile", id: saved.id };
    toast(`Đã lưu “${saved.name}”`);
    saving = false;
    render();
    return true;
  } catch (e) {
    saving = false;
    saveError = errorText(e);
    // Lỗi của một góc: chuyển tới góc đó.
    const m = /^Góc (Bắc|Đông Bắc|Đông|Đông Nam|Nam|Tây Nam|Tây|Tây Bắc):/.exec(saveError);
    if (m && editor) {
      const names = ["Bắc", "Đông Bắc", "Đông", "Đông Nam", "Nam", "Tây Nam", "Tây", "Tây Bắc"];
      editor.selectDir(names.indexOf(m[1]));
    }
    renderSavebar();
    return false;
  }
}

function revertDraft() {
  saveError = null;
  if (!draft) return;
  if (!draft.originalId) {
    draft = null;
    page = { kind: "profile", id: "default" };
    render();
    return;
  }
  const original = app.profiles.find((p) => p.id === draft?.originalId);
  if (original) {
    const selected = draft.selected;
    draft = makeDraft(original);
    draft.selected = selected;
  }
  render();
}

// ---------- Điều hướng ----------

async function navigate(next: Page) {
  if (JSON.stringify(next) === JSON.stringify(page)) return;
  if (draft && (isDirty(draft) || !draft.originalId)) {
    const name = draft.profile.name || "profile mới";
    const choice = await dialog(
      `Lưu thay đổi của “${name}”?`,
      "Nếu không lưu, thay đổi vừa làm sẽ mất.",
      [
        { label: "Ở lại", value: "stay" as const },
        { label: "Bỏ thay đổi", value: "discard" as const, kind: "ghost" },
        { label: "Lưu", value: "save" as const, kind: "primary" },
      ],
      "stay" as const,
    );
    if (choice === "stay") return;
    if (choice === "save" && !(await saveDraft())) return;
  }
  saveError = null;
  page = next;
  draft = null;
  render();
  main.scrollTo({ top: 0 });
  main.focus({ preventScroll: true });
}

// ---------- Vẽ trang ----------

const pageHost: PageHost = {
  app: () => app,
  updateSettings,
  suggestions,
  refresh: async () => {
    app = await api.state();
    render();
  },
  rerender: () => render(),
};

function render() {
  applyTheme(app.settings);
  editor = null;
  logView = null;
  let content: HTMLElement;

  if (page.kind === "profile" || page.kind === "new") {
    if (page.kind === "profile") {
      const profile = app.profiles.find((p) => p.id === (page as { id: string }).id) ?? app.profiles.find((p) => p.id === "default") ?? app.profiles[0];
      if (!profile) {
        content = h("div", { class: "page" }, h("p", { class: "empty" }, "Chưa có profile nào. Vào Sao lưu & khôi phục để khôi phục bộ mặc định."));
        mount(content);
        return;
      }
      page = { kind: "profile", id: profile.id };
      if (!draft || draft.originalId !== profile.id) draft = makeDraft(profile);
    }
    if (!draft) {
      page = { kind: "profile", id: "default" };
      return render();
    }
    editor = new ProfileEditor(draft, {
      app: () => app,
      suggestions,
      changed: () => {
        saveError = null;
        renderSavebar();
        renderSide();
      },
      remove: removeProfile,
      resetToBuiltin: () => {
        const b = app.builtins.find((x) => x.id === draft?.profile.id);
        if (!b || !draft) return;
        draft.profile = structuredClone(b);
        draft.profile.match.class ??= [];
        render();
        toast("Đã đưa về bản gốc. Bấm Lưu để giữ.", "info");
      },
    });
    content = editor.el;
  } else if (page.kind === "behavior") {
    content = behaviorPage(pageHost);
  } else if (page.kind === "appearance") {
    content = appearancePage(pageHost);
  } else if (page.kind === "log") {
    logView = logPage(pageHost, logEntries, () => (logEntries.length = 0));
    content = logView.el;
  } else {
    content = backupPage(pageHost);
  }
  mount(content);
}

function mount(content: HTMLElement) {
  const banners: HTMLElement[] = [];
  if (app.hookError) banners.push(h("p", { class: "banner danger", role: "alert" }, `Chưa cài được hook chuột, menu tròn sẽ không mở: ${app.hookError}`));
  if (app.loadErrors.length && page.kind !== "backup") {
    banners.push(
      h(
        "p",
        { class: "banner warn" },
        `Có ${app.loadErrors.length} lỗi khi đọc cấu hình. `,
        h("button", { type: "button", class: "link-btn", onclick: () => navigate({ kind: "backup" }) }, "Xem chi tiết"),
      ),
    );
  }
  main.replaceChildren(...banners, content);
  renderSide();
  renderSavebar();
}

async function removeProfile() {
  if (!draft?.originalId) return revertDraft();
  const name = draft.profile.name;
  const ok = await dialog(`Xoá profile “${name}”?`, "File profile sẽ bị xoá. Các app của nó sẽ dùng profile Mặc định.", [{ label: "Huỷ", value: false }, { label: "Xoá", value: true, kind: "danger" }], false);
  if (!ok) return;
  try {
    suppressUntil = Date.now() + 1500;
    await api.deleteProfile(draft.originalId);
    app = await api.state();
    draft = null;
    page = { kind: "profile", id: "default" };
    render();
    toast(`Đã xoá “${name}”`);
  } catch (e) {
    toast(errorText(e), "error");
  }
}

// ---------- Cài đặt chung: lưu ngay ----------

let settingsTimer: number | undefined;

function updateSettings(patch: Partial<Settings>, delay = 0) {
  Object.assign(app.settings, patch);
  applyTheme(app.settings);
  window.clearTimeout(settingsTimer);
  settingsTimer = window.setTimeout(async () => {
    suppressUntil = Date.now() + 1500;
    try {
      app.settings = await api.saveSettings(app.settings);
      applyTheme(app.settings);
    } catch (e) {
      toast(errorText(e), "error");
      app = await api.state();
      render();
    }
  }, delay);
}

// ---------- Thêm ứng dụng ----------

function slugify(process: string): string {
  const base = process
    .toLowerCase()
    .replace(/\.exe$/, "")
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/đ/g, "d")
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 34) || "app";
  let id = base;
  for (let n = 2; app.profiles.some((p) => p.id === id); n++) id = `${base}-${n}`;
  return id;
}

/** Tên dễ đọc: phần cuối tiêu đề cửa sổ ("Untitled - Notepad" → "Notepad"), không thì tên file. */
function niceName(process: string, title?: string): string {
  const fromTitle = title?.split(/ [-–—] /).pop()?.trim();
  if (fromTitle && fromTitle.length <= 28) return fromTitle;
  const stem = process.replace(/\.exe$/i, "");
  return stem.charAt(0).toUpperCase() + stem.slice(1);
}

async function addAppDialog() {
  const apps = await api.listApps().catch(() => [] as AppInfo[]);
  const rows = new Map<string, string>();
  for (const a of apps) rows.set(a.process, a.title);
  for (const r of app.recent) if (!rows.has(r)) rows.set(r, "vừa dùng menu ở đây");

  const owner = (proc: string) => app.profiles.find((p) => p.match.process.includes(proc) && !(p.match.class ?? []).length);
  let chosen = "";
  let chosenTitle: string | undefined;

  const dlg = h("dialog", { class: "dialog wide", "aria-labelledby": "add-title" });
  const procInput = h("input", { type: "text", class: "input mono", placeholder: "notepad.exe", spellcheck: "false", id: "add-proc", autocomplete: "off" });
  const list = h("ul", { class: "app-list", role: "listbox", "aria-label": "App đang mở" });
  let fromId = "";
  const from = dropdown<string>({
    options: [["", "Trống: tự đặt 8 góc"], ...sortedProfiles(app.profiles).map((p) => [p.id, `Sao chép từ “${p.name}”`] as const)],
    value: "",
    label: "Bắt đầu từ",
    id: "add-from",
    className: "wide",
    onChange: (v) => (fromId = v),
  });
  const createBtn = h("button", { type: "button", class: "btn primary", disabled: true }, "Tạo profile");
  const note = h("p", { class: "hint warn" });

  const sync = () => {
    chosen = procInput.value.trim().toLowerCase();
    if (chosen && !chosen.includes(".")) chosen += ".exe";
    createBtn.disabled = !chosen;
    const o = chosen ? owner(chosen) : undefined;
    note.textContent = o ? `${chosen} đang dùng profile “${o.name}”. Profile mới sẽ trùng điều kiện với nó.` : "";
    for (const li of list.querySelectorAll<HTMLElement>("[data-proc]")) {
      const match = !procInput.value || li.dataset.proc!.includes(procInput.value.trim().toLowerCase());
      li.hidden = !match;
      li.setAttribute("aria-selected", String(li.dataset.proc === chosen));
    }
  };

  for (const [proc, title] of [...rows].sort((a, b) => a[0].localeCompare(b[0]))) {
    const o = owner(proc);
    const btn = h(
      "button",
      { type: "button", class: "app-row" },
      h("span", { class: "mono app-proc" }, proc),
      h("span", { class: "app-title" }, title),
      o ? h("span", { class: "badge muted" }, o.name) : null,
    );
    btn.addEventListener("click", () => {
      procInput.value = proc;
      chosenTitle = title;
      sync();
    });
    btn.addEventListener("dblclick", () => create());
    list.append(h("li", { "data-proc": proc, role: "option" }, btn));
  }
  procInput.addEventListener("input", () => {
    chosenTitle = rows.get(procInput.value.trim().toLowerCase());
    sync();
  });
  procInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && chosen) create();
  });

  const close = () => {
    dlg.close();
    dlg.remove();
  };

  async function create() {
    if (!chosen) return;
    const base = app.profiles.find((p) => p.id === fromId);
    const fresh: Profile = {
      id: slugify(chosen),
      name: niceName(chosen, chosenTitle),
      icon: base?.icon ?? "star",
      match: { process: [chosen], class: [] },
      enabled: true,
      slots: base ? structuredClone(base.slots) : [],
    };
    close();
    const ok = await (async () => {
      if (draft && (isDirty(draft) || !draft.originalId)) {
        await navigate({ kind: "behavior" }); // hỏi lưu bản nháp đang dở
        return !draft;
      }
      return true;
    })();
    if (!ok) return;
    draft = makeDraft(fresh, true);
    page = { kind: "new" };
    saveError = null;
    render();
    (main.querySelector(".title-input") as HTMLInputElement | null)?.select();
  }

  createBtn.addEventListener("click", create);
  dlg.append(
    h("h2", { id: "add-title" }, "Thêm ứng dụng"),
    h("p", { class: "muted" }, "Chọn một app đang mở, hoặc gõ tên file chương trình của nó."),
    h("div", { class: "field" }, h("label", { for: "add-proc" }, "Tên file chương trình"), procInput),
    rows.size ? list : h("p", { class: "hint" }, "Không đọc được danh sách app đang mở. Gõ tên file, ví dụ notepad.exe (xem trong Task Manager › Details)."),
    note,
    h("div", { class: "field" }, h("label", { for: "add-from" }, "Bắt đầu từ"), from),
    h("div", { class: "dialog-actions" }, h("button", { type: "button", class: "btn ghost", onclick: close }, "Huỷ"), createBtn),
  );
  dlg.addEventListener("cancel", (e) => {
    e.preventDefault();
    close();
  });
  document.body.append(dlg);
  dlg.showModal();
  procInput.focus();
}

// ---------- Sự kiện từ lõi ----------

function onGesture(e: GestureEvent) {
  const d = describeEvent(e);
  if (e.kind === "open" && e.process && !app.recent.includes(e.process)) app.recent = [e.process, ...app.recent].slice(0, 12);
  if (!d) return;
  const now = new Date();
  const entry: LogEntry = { ...d, time: `${now.toLocaleTimeString("vi-VN", { hour12: false })}.${String(now.getMilliseconds()).padStart(3, "0")}` };
  logEntries.unshift(entry);
  if (logEntries.length > 200) logEntries.length = 200;
  logView?.add(entry);
}

async function onConfigChanged() {
  const fresh = await api.state();
  app = fresh;
  if (Date.now() < suppressUntil) {
    renderSide();
    return;
  }
  // Có bản nháp đang sửa dở thì giữ nguyên, chỉ cập nhật danh sách.
  if (draft && (isDirty(draft) || !draft.originalId)) {
    renderSide();
    return;
  }
  draft = null;
  render();
}

// Cửa sổ là app, không phải trang web: bỏ menu chuột phải và phím tắt của trình duyệt
// (tải lại, in, tìm, quay lại…). Ô ghi phím chặn sự kiện trước nên vẫn ghi được các phím này.
document.addEventListener("contextmenu", (e) => e.preventDefault());
document.addEventListener("auxclick", (e) => e.button > 1 && e.preventDefault());
document.addEventListener("mouseup", (e) => (e.button === 3 || e.button === 4) && e.preventDefault());
document.addEventListener("keydown", (e) => {
  const k = e.key.toLowerCase();
  const ctrl = e.ctrlKey || e.metaKey;
  if (
    k === "f5" ||
    k === "f3" ||
    k === "browserback" ||
    k === "browserforward" ||
    (e.altKey && (k === "arrowleft" || k === "arrowright")) ||
    (ctrl && ["r", "p", "f", "g", "j", "u", "h", "o", "n", "t"].includes(k))
  ) {
    e.preventDefault();
  }
});

document.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
    e.preventDefault();
    if (draft && (isDirty(draft) || !draft.originalId)) saveDraft();
  }
});

// ---------- Khởi động ----------

async function init() {
  main.replaceChildren(h("p", { class: "loading" }, "Đang khởi động…"));
  app = await api.state();
  // Cửa sổ có thể tải xong trước khi lõi nạp cấu hình và cài hook: hỏi lại một lúc.
  for (let i = 0; app.starting && i < 40; i++) {
    await sleep(250);
    app = await api.state();
  }
  applyTheme(app.settings);
  render();

  await listen<GestureEvent>("gesture", (ev) => onGesture(ev.payload));
  await listen("config-changed", () => void onConfigChanged());
  await listen<boolean>("paused", (ev) => {
    app.paused = ev.payload;
    renderSide();
  });
}

init().catch((err) => {
  main.replaceChildren(h("p", { class: "banner danger" }, `Không kết nối được với lõi app: ${errorText(err)}`));
});
