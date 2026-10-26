// Các trang chung: Cử chỉ & khởi động, Giao diện, Nhật ký, Sao lưu & khôi phục.
// Cài đặt ở đây lưu ngay khi đổi (không cần bấm Lưu).
import { DIRS } from "../shared/geometry";
import type { AppState, GestureEvent, Settings } from "../shared/types";
import { api, errorText } from "./api";
import { chip, chipAdder, h, icon, segmented, toggle } from "./dom";
import { confirmDialog, dialog, toast } from "./overlay";
import { RadialPreview } from "./preview";

export interface PageHost {
  app(): AppState;
  /** Đổi cài đặt; `delay` > 0 thì gộp các lần đổi liên tiếp (thanh trượt). */
  updateSettings(patch: Partial<Settings>, delay?: number): void;
  suggestions(): Promise<{ value: string; note: string }[]>;
  /** Đọc lại trạng thái từ lõi rồi vẽ lại. */
  refresh(): Promise<void>;
  /** Vẽ lại trang hiện tại từ dữ liệu đang có. */
  rerender(): void;
}

function card(title: string, ...children: (Node | null)[]): HTMLElement {
  return h("section", { class: "card" }, h("h2", { class: "card-title" }, title), ...children);
}

function pageHead(title: string, sub: string): HTMLElement {
  return h("header", { class: "page-head" }, h("h1", null, title), h("p", { class: "sub" }, sub));
}

function slider(
  label: string,
  value: number,
  min: number,
  max: number,
  hint: string,
  onInput: (v: number) => void,
  step = 1,
  format: (v: number) => string = (v) => `${v} px`,
): HTMLElement {
  const out = h("output", { class: "mono value" }, format(value));
  const input = h("input", { type: "range", min, max, step, value, class: "range" });
  // Phần đã kéo tô màu nhấn (thanh trượt tự vẽ).
  const fill = () => input.style.setProperty("--pct", `${((Number(input.value) - min) / (max - min)) * 100}%`);
  fill();
  input.addEventListener("input", () => {
    fill();
    out.textContent = format(Number(input.value));
    onInput(Number(input.value));
  });
  const id = `rng-${label.length}-${min}`;
  input.id = id;
  return h("div", { class: "field" }, h("div", { class: "field-top" }, h("label", { for: id }, label), out), input, h("p", { class: "hint" }, hint));
}

// ---------- Cử chỉ & khởi động ----------

/** Bật/tắt tự đóng khi kéo quá xa, kèm thanh chỉnh khoảng cách (mờ đi khi tắt). */
function farControls(host: PageHost): HTMLElement {
  const s = host.app().settings;
  const distance = slider(
    "Khoảng cách tự đóng",
    s.closeDistancePx,
    300,
    1000,
    "Tính từ chỗ nhấn; mép menu cách tâm khoảng 170 px. Cú vụt nhanh ngay lúc mở (nhấn, vụt, nhả) không bị tính.",
    (v) => host.updateSettings({ closeDistancePx: v }, 300),
    10,
  );
  const setEnabled = (on: boolean) => {
    distance.classList.toggle("disabled", !on);
    distance.querySelector("input")?.toggleAttribute("disabled", !on);
  };
  setEnabled(s.closeWhenFar);
  const sw = toggle("Kéo ra quá xa thì đóng menu", s.closeWhenFar, (on) => {
    host.updateSettings({ closeWhenFar: on });
    setEnabled(on);
  }, "Đổi ý giữa chừng: kéo vụt ra ngoài là menu đóng, nhả chuột không chạy gì.");
  return h("div", { class: "grid-2 far" }, sw, distance);
}

export function behaviorPage(host: PageHost): HTMLElement {
  const s = host.app().settings;

  const excludeChips = h("div", { class: "chips", role: "list", "aria-label": "App không mở menu tròn" });
  const drawExclude = () => {
    const list = host.app().settings.exclude;
    excludeChips.replaceChildren(
      ...list.map((p, i) =>
        chip(p, () => {
          const next = list.filter((_, j) => j !== i);
          host.updateSettings({ exclude: next });
          drawExclude();
        }),
      ),
      chipAdder("+ Thêm app", "game.exe", (v) => {
        const value = v.toLowerCase();
        const cur = host.app().settings.exclude;
        if (!cur.includes(value)) host.updateSettings({ exclude: [...cur, value] });
        drawExclude();
      }, () => host.suggestions()),
    );
  };
  drawExclude();

  return h(
    "div",
    { class: "page" },
    pageHead("Cử chỉ & khởi động", "Kéo bao xa thì mở menu tròn, khi nào để nguyên chuột phải của Windows, và chạy cùng Windows."),
    card(
      "Cử chỉ",
      h(
        "div",
        { class: "grid-2" },
        slider("Ngưỡng kéo", s.thresholdPx, 4, 30, "Nhả chuột trước khi kéo đủ xa thì ra menu chuột phải gốc của Windows.", (v) =>
          host.updateSettings({ thresholdPx: v }, 300),
        ),
        slider("Vùng huỷ ở giữa", s.cancelRadiusPx, 10, 80, "Nhả chuột trong vòng tròn này (tính từ chỗ nhấn) thì đóng menu, không chạy gì.", (v) =>
          host.updateSettings({ cancelRadiusPx: v }, 300),
        ),
      ),
      farControls(host),
      h(
        "ul",
        { class: "tips" },
        h("li", null, "Giữ ", h("kbd", null, "Shift"), " khi nhấn chuột phải để luôn ra menu gốc."),
        h("li", null, "Đang mở menu, bấm ", h("kbd", null, "Esc"), " hoặc nút chuột khác để huỷ."),
      ),
    ),
    card(
      "Không mở menu tròn khi",
      toggle("Có app chạy toàn màn hình", s.disableWhenFullscreen, (v) => host.updateSettings({ disableWhenFullscreen: v }), "Game, video, trình chiếu đang ở chế độ toàn màn hình."),
      h(
        "div",
        { class: "field" },
        h("span", { class: "field-label" }, "Trong các app này"),
        excludeChips,
        h("p", { class: "hint" }, "App chạy quyền Admin cũng được bỏ qua tự động, vì Windows không cho gửi phím vào đó."),
      ),
    ),
    card(
      "Khởi động",
      toggle("Chạy cùng Windows", s.launchAtLogin, (v) => host.updateSettings({ launchAtLogin: v }), "Tự mở khi đăng nhập, nằm im dưới khay hệ thống."),
    ),
    h(
      "div",
      { class: "page-foot" },
      h("button", {
        type: "button",
        class: "btn ghost",
        onclick: async () => {
          if (!(await confirmDialog("Đặt lại cử chỉ?", "Ngưỡng kéo, vùng huỷ, tự đóng khi kéo xa, danh sách app bỏ qua và chế độ toàn màn hình về như lúc mới cài.", "Đặt lại"))) return;
          const d = host.app().defaults;
          host.updateSettings({
            thresholdPx: d.thresholdPx,
            cancelRadiusPx: d.cancelRadiusPx,
            closeWhenFar: d.closeWhenFar,
            closeDistancePx: d.closeDistancePx,
            exclude: [...d.exclude],
            disableWhenFullscreen: d.disableWhenFullscreen,
          });
          host.rerender();
        },
      }, icon("undo", 15), "Đặt lại mặc định"),
    ),
  );
}

// ---------- Giao diện ----------

export function appearancePage(host: PageHost): HTMLElement {
  const app = host.app();
  const s = app.settings;
  const sample = app.profiles.find((p) => p.id === "browser") ?? app.profiles[0] ?? app.builtins[0];
  const preview = new RadialPreview({ interactive: false });
  // Bản xem trước to nhỏ theo kích thước menu (225 px ứng với 100%).
  const sizePreview = (scale: number) => (preview.el.style.width = `${Math.round(225 * scale)}px`);
  sizePreview(s.menuScale);
  let demo = 1;
  if (sample) preview.update(sample, demo);
  // Vùng sáng chạy vòng quanh để thấy màu nhấn trên menu.
  const cycle = window.setInterval(() => {
    if (!preview.el.isConnected) return window.clearInterval(cycle);
    if (sample && !document.hidden) preview.update(sample, (demo = (demo + 1) % 8));
  }, 1400);

  const swatches = h("div", { class: "swatches", role: "radiogroup", "aria-label": "Màu nhấn" });
  const drawSwatches = () => {
    const current = host.app().settings.accent.toUpperCase();
    const isCustom = !app.accents.includes(current);
    swatches.replaceChildren(
      ...app.accents.map((hex) => {
        const b = h("button", { type: "button", class: "swatch", role: "radio", "aria-checked": String(hex === current), "aria-label": `Màu ${hex}`, style: `--sw: ${hex}` });
        b.addEventListener("click", () => {
          host.updateSettings({ accent: hex });
          drawSwatches();
        });
        return b;
      }),
    );
    const picker = h("input", { type: "color", value: current.toLowerCase(), "aria-label": "Màu tuỳ chọn" });
    picker.addEventListener("input", () => host.updateSettings({ accent: picker.value.toUpperCase() }, 200));
    picker.addEventListener("change", drawSwatches);
    swatches.append(h("label", { class: `swatch custom${isCustom ? " on" : ""}`, title: "Màu khác" }, picker, icon("plus", 14)));
  };
  drawSwatches();

  return h(
    "div",
    { class: "page" },
    pageHead("Giao diện", "Màu sắc và hiệu ứng của menu tròn và cửa sổ này."),
    h(
      "div",
      { class: "editor-grid" },
      h("section", { class: "card preview-card", "aria-label": "Xem trước" }, h("p", { class: "card-note" }, "Xem trước"), preview.el),
      h(
        "section",
        { class: "card stack-lg" },
        segmented("tone", "Tông màu", [["dark", "Tối"], ["light", "Sáng"]] as const, s.tone, (v) => host.updateSettings({ tone: v })),
        h("div", { class: "field" }, h("span", { class: "field-label" }, "Màu nhấn"), swatches),
        slider(
          "Kích thước menu",
          Math.round(s.menuScale * 100),
          80,
          140,
          "100% là đĩa menu rộng khoảng 344 px (chưa tính scale của Windows). Áp dụng từ lần mở menu tiếp theo.",
          (v) => {
            sizePreview(v / 100);
            host.updateSettings({ menuScale: v / 100 }, 300);
          },
          5,
          (v) => `${v}%`,
        ),
        segmented("anim", "Hiệu ứng chuyển động", [["system", "Theo Windows"], ["on", "Luôn bật"], ["off", "Tắt"]] as const, s.animations, (v) =>
          host.updateSettings({ animations: v }),
        ),
        h("hr", { class: "sep" }),
        toggle("Làm mờ nội dung phía sau menu", s.blur, (v) => host.updateSettings({ blur: v }), "Kính mờ thật: chụp vùng dưới menu rồi làm mờ. Tắt nếu máy yếu."),
        toggle(
          "Ẩn menu khỏi ảnh chụp màn hình",
          s.hideFromCapture,
          (v) => host.updateSettings({ hideFromCapture: v }),
          "Tắt khi cần chụp hoặc quay lại menu. Khi tắt, nền mờ phải chụp trước lúc menu hiện nên chậm hơn một chút.",
        ),
      ),
    ),
  );
}

// ---------- Nhật ký ----------

export interface LogEntry {
  time: string;
  label: string;
  detail: string;
  tone: "" | "accent" | "muted" | "warn" | "danger" | "quiet";
}

const dirText = (d: number | null) => (d === null ? "giữa (vùng huỷ)" : `${DIRS[d].name} ${DIRS[d].arrow}`);

export function describeEvent(e: GestureEvent): Omit<LogEntry, "time"> | null {
  switch (e.kind) {
    case "open":
      return { label: "Mở", detail: `${e.process ?? "?"} → ${e.profile}`, tone: "" };
    case "dir":
      return null; // quá dày, không ghi
    case "cancel":
      return { label: "Huỷ", detail: "nhả ở giữa, bấm Esc hoặc nút khác", tone: "muted" };
    case "away":
      return { label: "Đóng menu", detail: "kéo ra quá xa", tone: "muted" };
    case "empty":
      return { label: "Góc trống", detail: dirText(e.dir), tone: "muted" };
    case "unavailable":
      return { label: "Góc đang tắt", detail: `${e.label}: ${e.reason}`, tone: "muted" };
    case "run":
      return { label: "Chạy", detail: `${e.label} · ${e.detail}`, tone: "accent" };
    case "error":
      return { label: "Lỗi", detail: `${e.label}: ${e.message}`, tone: "danger" };
    case "native":
      return { label: "Menu gốc", detail: "nhả trước ngưỡng, trả lại chuột phải cho Windows", tone: "muted" };
    case "handover":
      return { label: "Trả cho Windows", detail: "có nút khác được nhấn khi đang giữ", tone: "muted" };
    case "skip":
      return { label: "Bỏ qua", detail: `${e.process}: ${e.reason}`, tone: "muted" };
    case "latency":
      return { label: "Menu hiện", detail: `sau ${e.ms.toFixed(1)} ms`, tone: e.ms <= 16 ? "quiet" : "warn" };
    case "loaded":
      return { label: "Nạp cấu hình", detail: `${e.profiles} profile${e.errors.length ? ` · ${e.errors.length} lỗi` : ""}`, tone: e.errors.length ? "warn" : "quiet" };
  }
}

export function logRow(entry: LogEntry): HTMLLIElement {
  return h(
    "li",
    { class: `log-row ${entry.tone}` },
    h("time", null, entry.time),
    h("span", { class: "log-label" }, entry.label),
    h("span", { class: "log-detail", title: entry.detail }, entry.detail),
  );
}

export function logPage(host: PageHost, entries: LogEntry[], onClear: () => void): { el: HTMLElement; add(entry: LogEntry): void } {
  const app = host.app();
  const list = h("ol", { class: "log", "aria-live": "polite" }, ...entries.map(logRow));
  const empty = h("p", { class: "empty" }, "Chưa có gì. Thử nhấn giữ chuột phải ở bất kỳ đâu rồi kéo.");
  empty.hidden = entries.length > 0;
  const status = app.hookError
    ? h("p", { class: "banner danger" }, `Chưa cài được hook chuột: ${app.hookError}`)
    : h("p", { class: "sub" }, app.paused ? "Đang tạm dừng: chuột phải hoạt động như bình thường." : "Hook chuột đang hoạt động. Mỗi lần mở menu được ghi lại ở đây.");
  const el = h(
    "div",
    { class: "page" },
    pageHead("Nhật ký", "Những gì Radial Menu vừa làm, để kiểm tra khi một lệnh không chạy như ý."),
    h(
      "section",
      { class: "card log-card" },
      h(
        "div",
        { class: "card-head" },
        status,
        h("button", {
          type: "button",
          class: "btn ghost sm",
          onclick: () => {
            onClear();
            list.replaceChildren();
            empty.hidden = false;
          },
        }, "Xoá nhật ký"),
      ),
      list,
      empty,
    ),
  );
  return {
    el,
    add(entry) {
      list.prepend(logRow(entry));
      while (list.children.length > 200) list.lastElementChild?.remove();
      empty.hidden = true;
    },
  };
}

// ---------- Sao lưu & khôi phục ----------

export function backupPage(host: PageHost): HTMLElement {
  const app = host.app();

  const fileInput = h("input", { type: "file", accept: ".json,application/json", class: "visually-hidden", tabindex: "-1", "aria-hidden": "true" });
  fileInput.addEventListener("change", async () => {
    const file = fileInput.files?.[0];
    fileInput.value = "";
    if (!file) return;
    let text: string;
    try {
      text = await file.text();
    } catch (e) {
      toast(`Không đọc được file: ${errorText(e)}`, "error");
      return;
    }
    // Đọc thử để nói trước sẽ nhập những gì.
    let summary = "Nội dung trong file sẽ được nhập.";
    try {
      const data = JSON.parse(text.replace(/^﻿/, ""));
      const names: string[] = Array.isArray(data.profiles) ? data.profiles.map((p: { name?: string; id?: string }) => p.name || p.id || "?") : data.slots ? [data.name || data.id] : [];
      summary = [
        names.length ? `${names.length} profile: ${names.slice(0, 6).join(", ")}${names.length > 6 ? "…" : ""}.` : "",
        data.settings ? "Cả cài đặt chung (trừ Chạy cùng Windows)." : "",
        "Profile trùng mã sẽ bị thay bằng bản trong file.",
      ]
        .filter(Boolean)
        .join(" ");
    } catch {
      /* Rust sẽ báo lỗi cụ thể */
    }
    const ok = await dialog(`Nhập từ ${file.name}?`, summary, [{ label: "Huỷ", value: false }, { label: "Nhập", value: true, kind: "primary" }], false);
    if (!ok) return;
    try {
      const r = await api.importBackup(text);
      toast(`Đã nhập ${r.profiles.length} profile${r.settings ? " và cài đặt" : ""}`);
      await host.refresh();
    } catch (e) {
      toast(errorText(e), "error");
    }
  });

  const errors = app.loadErrors.length
    ? h(
        "section",
        { class: "card warn-card" },
        h("h2", { class: "card-title" }, "Lỗi khi đọc cấu hình"),
        h("ul", { class: "errors" }, ...app.loadErrors.map((e) => h("li", { class: "mono" }, e))),
        h("p", { class: "hint" }, "File lỗi bị bỏ qua, các file khác vẫn dùng bình thường. Sửa file rồi bấm Nạp lại."),
      )
    : null;

  return h(
    "div",
    { class: "page" },
    pageHead("Sao lưu & khôi phục", "Mang bộ lệnh sang máy khác, hoặc quay lại như lúc mới cài."),
    errors,
    card(
      "Sao lưu",
      h("p", { class: "muted" }, "Một file JSON gồm cài đặt chung và mọi profile. Nhập lại được cả file profile lẻ trong thư mục cấu hình."),
      h(
        "div",
        { class: "row-actions" },
        h("button", {
          type: "button",
          class: "btn primary",
          onclick: async () => {
            try {
              const path = await api.exportBackup();
              toast(`Đã lưu ${path.split(/[\\/]/).pop()} vào ${path.includes("Downloads") ? "Downloads" : "thư mục cấu hình"}`);
            } catch (e) {
              toast(errorText(e), "error");
            }
          },
        }, icon("download", 16), "Xuất bản sao lưu"),
        h("button", { type: "button", class: "btn ghost", onclick: () => fileInput.click() }, icon("upload", 16), "Nhập từ file…"),
        fileInput,
      ),
    ),
    card(
      "Khôi phục",
      h("p", { class: "muted" }, "Ghi lại 5 profile có sẵn (Trình duyệt, File Explorer, Desktop, Code editor, Mặc định) về bản gốc. Profile bạn tự thêm được giữ nguyên."),
      h(
        "div",
        { class: "row-actions" },
        h("button", {
          type: "button",
          class: "btn ghost danger-text",
          onclick: async () => {
            if (!(await confirmDialog("Khôi phục profile mặc định?", "Thay đổi bạn đã làm trên các profile có sẵn sẽ mất. Profile tự thêm không bị ảnh hưởng.", "Khôi phục", true))) return;
            try {
              await api.restoreDefaults();
              toast("Đã khôi phục profile mặc định");
              await host.refresh();
            } catch (e) {
              toast(errorText(e), "error");
            }
          },
        }, icon("undo", 16), "Khôi phục profile mặc định"),
      ),
    ),
    card(
      "Thư mục cấu hình",
      h("p", { class: "path mono" }, app.configDir || "—"),
      h("p", { class: "hint" }, "Mỗi profile là một file .json trong thư mục profiles. Sửa tay xong bấm Nạp lại để áp dụng."),
      h(
        "div",
        { class: "row-actions" },
        h("button", { type: "button", class: "btn ghost", onclick: () => api.openConfigDir() }, icon("folder", 16), "Mở thư mục"),
        h("button", {
          type: "button",
          class: "btn ghost",
          onclick: async () => {
            await api.reload();
            await host.refresh();
            toast("Đã nạp lại cấu hình");
          },
        }, icon("refresh", 16), "Nạp lại"),
      ),
    ),
  );
}

