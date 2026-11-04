// Cập nhật: hộp thoại xem bản mới, tải (có thanh tiến độ) rồi chạy trình cài đặt.
// Lõi Rust (updater.rs) làm phần tải và kiểm tra chữ ký.
import { listen } from "@tauri-apps/api/event";
import type { UpdateInfo } from "../shared/types";
import { api, errorText } from "./api";
import { h, icon } from "./dom";
import { toast } from "./overlay";

/** Ghi chú phát hành (Markdown đơn giản) → đoạn văn và danh sách gạch đầu dòng. */
function notesView(text: string): HTMLElement {
  const box = h("div", { class: "update-notes" });
  const plain = (s: string) =>
    s
      .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
      .replace(/\*\*|__|`/g, "")
      .trim();
  let list: HTMLUListElement | null = null;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("<")) {
      list = null;
      continue;
    }
    const bullet = /^[-*]\s+(.*)$/.exec(line);
    if (bullet) {
      if (!list) box.append((list = h("ul")));
      list.append(h("li", null, plain(bullet[1])));
      continue;
    }
    list = null;
    const heading = /^#+\s+(.*)$/.exec(line);
    box.append(heading ? h("h3", null, plain(heading[1])) : h("p", null, plain(line)));
  }
  if (!box.childElementCount) box.append(h("p", null, "Bản sửa lỗi và cải thiện nhỏ."));
  return box;
}

const mb = (n: number) => `${(n / 1048576).toFixed(1)} MB`;

let open = false;

/** Hộp thoại bản mới: xem ghi chú, bấm cập nhật để tải và cài. */
export function updateDialog(info: UpdateInfo) {
  if (open) return;
  open = true;
  const previous = document.activeElement as HTMLElement | null;
  const dlg = h("dialog", { class: "dialog update-dialog", "aria-labelledby": "upd-title" });
  const body = h("div", { class: "update-body" });
  const actions = h("div", { class: "dialog-actions" });
  let busy = false;
  const unlisten: (() => void)[] = [];

  const close = () => {
    if (busy) return;
    unlisten.forEach((f) => f());
    dlg.close();
    dlg.remove();
    open = false;
    previous?.focus?.();
  };

  const later = h("button", { type: "button", class: "btn ghost", onclick: close }, "Để sau");
  const go = h("button", { type: "button", class: "btn primary" }, icon("download", 16), "Cập nhật ngay");

  const meta = h(
    "p",
    { class: "update-meta" },
    `Bạn đang dùng ${info.current}`,
    info.date ? ` · bản ${info.version} phát hành ngày ${info.date.split("-").reverse().join("/")}` : "",
  );
  body.append(meta, notesView(info.notes));

  const bar = h("div", { class: "update-bar", role: "progressbar", "aria-valuemin": "0", "aria-valuemax": "100" }, h("span"));
  const status = h("p", { class: "update-status", "aria-live": "polite" });

  go.addEventListener("click", async () => {
    busy = true;
    go.disabled = true;
    later.disabled = true;
    body.replaceChildren(status, bar);
    status.textContent = "Đang tải bản mới…";
    unlisten.push(
      await listen<{ downloaded: number; total: number | null }>("update:progress", (e) => {
        const { downloaded, total } = e.payload;
        const pct = total ? Math.min(100, Math.round((downloaded / total) * 100)) : null;
        (bar.firstElementChild as HTMLElement).style.width = `${pct ?? 30}%`;
        bar.classList.toggle("indeterminate", pct === null);
        if (pct !== null) bar.setAttribute("aria-valuenow", String(pct));
        status.textContent = total ? `Đang tải… ${mb(downloaded)} / ${mb(total)}` : `Đang tải… ${mb(downloaded)}`;
      }),
      await listen("update:installing", () => {
        (bar.firstElementChild as HTMLElement).style.width = "100%";
        status.textContent = "Đã tải xong. Đang mở trình cài đặt, Windows có thể hỏi quyền Admin. Radial Menu sẽ tự mở lại sau khi cài.";
      }),
    );
    try {
      await api.installUpdate();
      // Trên Windows app đã thoát trước khi tới đây; nếu chưa thì báo đã xong.
      status.textContent = "Đã cài xong bản mới.";
    } catch (e) {
      busy = false;
      later.disabled = false;
      go.disabled = false;
      body.replaceChildren(h("p", { class: "update-error", role: "alert" }, errorText(e)), meta, notesView(info.notes));
      go.replaceChildren(icon("refresh", 16), "Thử lại");
    }
  });

  actions.append(later, go);
  dlg.append(h("h2", { id: "upd-title" }, `Có bản mới: Radial Menu ${info.version}`), body, actions);
  dlg.addEventListener("cancel", (e) => {
    e.preventDefault();
    close();
  });
  dlg.addEventListener("click", (e) => {
    if (e.target === dlg) close();
  });
  document.body.append(dlg);
  dlg.showModal();
  go.focus();
}

/** Nút "Kiểm tra bản mới": hỏi GitHub, có thì mở hộp thoại, không thì báo. */
export async function checkNow(onFound: (info: UpdateInfo | null) => void) {
  toast("Đang kiểm tra bản mới…", "info");
  try {
    const info = await api.checkUpdate();
    onFound(info);
    if (info) updateDialog(info);
    else toast("Bạn đang dùng bản mới nhất.");
  } catch (e) {
    toast(errorText(e), "error");
  }
}
