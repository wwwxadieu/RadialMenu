// Menu tròn chạy trong cửa sổ overlay trong suốt.
// Rust quyết định mọi thứ (mở, hướng, đóng); trang này chỉ vẽ và chạy animation.
// Animation dùng CSS transition trên transform/opacity, đổi trạng thái qua data-phase.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyAccent } from "../shared/color";
import { arc, C, point, R_ITEM, wedge } from "../shared/geometry";
import { ICONS } from "../shared/icons";
import type { MenuLook } from "../shared/types";
import "./style.css";

type Dir = number | null;

interface Slot {
  label: string;
  keys: string;
  icon: string;
  /** Có lý do = góc đang tắt (ví dụ Dán khi clipboard trống): hiện mờ, không chọn được. */
  unavailable?: string;
}

/** Icon thật của app dưới con trỏ, RGBA base64. */
interface AppIcon {
  width: number;
  height: number;
  data: string;
}

interface Profile {
  id: string;
  name: string;
  process: string;
  icon: string;
  /** Đúng 8 phần tử; null = góc trống. */
  slots: (Slot | null)[];
}

interface OpenPayload {
  seq: number;
  dir: Dir;
  profile: Profile;
  appIcon: AppIcon | null;
}

/** Ảnh nền đã làm mờ của vùng dưới đĩa menu, RGBA base64. */
interface BackdropPayload {
  seq: number;
  width: number;
  height: number;
  data: string;
}

type Phase = "idle" | "pre" | "open" | "pick" | "cancel";

const SVG = "http://www.w3.org/2000/svg";
const SETTLE_MS = 700; // sau mốc này, nút chuyển sang transition nhanh khi đổi hướng
const CLOSE_MS = 340; // khớp thời lượng animation đóng dài nhất

const root = document.getElementById("menu") as HTMLDivElement;

// ---------- DOM ----------

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls: string, parent: Element) {
  const node = document.createElement(tag);
  node.className = cls;
  parent.appendChild(node);
  return node;
}

function svg(parent: Element, size: number, cls = ""): SVGSVGElement {
  const node = document.createElementNS(SVG, "svg");
  node.setAttribute("width", String(size));
  node.setAttribute("height", String(size));
  node.setAttribute("viewBox", `0 0 ${size} ${size}`);
  if (cls) node.setAttribute("class", cls);
  parent.appendChild(node);
  return node;
}

function path(parent: Element, d: string, attrs: Record<string, string> = {}): SVGPathElement {
  const node = document.createElementNS(SVG, "path");
  node.setAttribute("d", d);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  parent.appendChild(node);
  return node;
}

function iconSvg(parent: Element, size: number): SVGPathElement {
  const node = svg(parent, 24);
  node.setAttribute("width", String(size));
  node.setAttribute("height", String(size));
  node.setAttribute("fill", "none");
  node.setAttribute("stroke", "currentColor");
  node.setAttribute("stroke-width", "2");
  node.setAttribute("stroke-linecap", "round");
  node.setAttribute("stroke-linejoin", "round");
  return path(node, "");
}

// ---------- Dựng DOM một lần ----------

// Gợn sóng lúc mở: quầng sáng mờ quanh điểm nhấn và một vòng sóng mềm lan ra.
const ripple = el("div", "ripple", root);
for (const cls of ["rp-bloom", "rp-ring"]) el("i", cls, ripple);

const glass = el("div", "glass", root);
const disc = el("div", "disc", glass);
// Nền kính: ảnh màn hình phía sau đã làm mờ (Rust gửi sang), rồi một lớp màu kính phủ lên.
const backdrop = el("canvas", "backdrop", disc);
el("div", "tint", disc);
path(svg(disc, 360, "tiles"), Array.from({ length: 8 }, (_, i) => wedge(i)).join(""));

const highlight = el("div", "hl", root);
const hlFill = el("div", "hl-fill", highlight);
hlFill.style.clipPath = `path('${wedge(0)}')`;
path(svg(highlight, 360), arc(0, 59, 19), { class: "hl-arc" });

interface ItemView {
  node: HTMLDivElement;
  icon: SVGPathElement;
  label: HTMLSpanElement;
}

const items: ItemView[] = Array.from({ length: 8 }, (_, i) => {
  const [x, y] = point(R_ITEM, -90 + i * 45);
  const node = el("div", "item", root);
  node.style.setProperty("--x", `${x.toFixed(1)}px`);
  node.style.setProperty("--y", `${y.toFixed(1)}px`);
  // Độ lệch về phía tâm: nút trượt từ đây ra khi mở và thu về đây khi đóng.
  node.style.setProperty("--ix", `${((C - x) * 0.28).toFixed(1)}px`);
  node.style.setProperty("--iy", `${((C - y) * 0.28).toFixed(1)}px`);
  node.style.setProperty("--d", `${70 + i * 22}ms`);
  node.style.setProperty("--dc", `${(7 - i) * 10}ms`);
  const icon = iconSvg(node, 20);
  const label = el("span", "", node);
  return { node, icon, label };
});

const hub = el("div", "hub", root);
const hubApp = el("div", "hub-app", hub);
const appIcon = iconSvg(hubApp, 22);
// Icon thật của app (thay icon profile khi có).
const appImage = el("canvas", "hub-appicon", hubApp);
hubApp.insertBefore(appImage, hubApp.firstChild);
const appName = el("span", "hub-name", hubApp);
const appProc = el("span", "hub-proc", hubApp);
const hubAct = el("div", "hub-act", hub);
const actLabel = el("span", "hub-name", hubAct);
const actKeys = el("span", "hub-keys", hubAct);

// ---------- Trạng thái ----------

let profile: Profile | null = null;
let currentSeq = -1;
let rotation: number | undefined;
let settleTimer: number | undefined;
let closeTimer: number | undefined;

function setPhase(phase: Phase) {
  root.dataset.phase = phase;
}

function render(p: Profile) {
  profile = p;
  items.forEach((item, i) => {
    const slot = p.slots[i];
    item.label.textContent = slot?.label ?? "";
    item.icon.setAttribute("d", ICONS[slot?.icon ?? ""] ?? "");
    item.node.hidden = !slot;
    item.node.classList.toggle("off", Boolean(slot?.unavailable));
  });
  appIcon.setAttribute("d", ICONS[p.icon] ?? ICONS.grid);
  appName.textContent = p.name;
  appProc.textContent = p.process;
}

function setDir(dir: Dir) {
  const off = dir !== null ? profile?.slots[dir]?.unavailable : undefined;
  root.dataset.hasDir = String(dir !== null);
  // Góc đang tắt: không sáng lên, giữa menu nói lý do.
  root.dataset.dirOff = String(Boolean(off));
  items.forEach((item, i) => item.node.classList.toggle("on", i === dir && !off));
  if (dir === null) return;

  // Xoay vùng sáng theo đường ngắn nhất (Tây Bắc → Bắc là +45°, không phải −315°).
  const target = dir * 45;
  if (rotation === undefined) {
    rotation = target;
  } else {
    const current = ((rotation % 360) + 360) % 360;
    let delta = target - current;
    if (delta > 180) delta -= 360;
    if (delta < -180) delta += 360;
    rotation += delta;
  }
  root.style.setProperty("--rot", `${rotation}deg`);

  const slot = profile?.slots[dir];
  actLabel.textContent = slot?.label ?? "Góc trống";
  actKeys.textContent = off ?? slot?.keys ?? "";
  actKeys.hidden = !actKeys.textContent;
  actKeys.classList.toggle("off", Boolean(off));
}

function open(payload: OpenPayload) {
  window.clearTimeout(settleTimer);
  window.clearTimeout(closeTimer);
  render(payload.profile);
  currentSeq = payload.seq;
  root.dataset.backdrop = "off";
  root.dataset.appicon = "off";
  if (payload.appIcon) paintAppIcon(payload.appIcon);

  // "pre" = trạng thái xuất phát của animation mở. Ép trình duyệt tính style ở đây
  // rồi mới chuyển sang "open", để transition chạy ngay trong khung hình đầu.
  root.dataset.settled = "false";
  setPhase("pre");
  rotation = undefined;
  setDir(payload.dir);
  void root.offsetWidth;
  setPhase("open");

  requestAnimationFrame(() => {
    invoke("menu_frame", { seq: payload.seq }).catch(() => {});
  });
  settleTimer = window.setTimeout(() => (root.dataset.settled = "true"), SETTLE_MS);
}

function close(mode: "pick" | "cancel", dir: Dir) {
  window.clearTimeout(settleTimer);
  if (mode === "pick") setDir(dir);
  else root.dataset.dirOff = "false";
  setPhase(mode);
  // Về trạng thái trong suốt trước khi Rust ẩn cửa sổ, để lần hiện sau không lộ khung cũ.
  closeTimer = window.setTimeout(() => {
    setPhase("idle");
    root.dataset.hasDir = "false";
    items.forEach((item) => item.node.classList.remove("on"));
  }, CLOSE_MS);
}

function decode(data: string, width: number, height: number): ImageData | null {
  const bytes = Uint8ClampedArray.from(atob(data), (c) => c.charCodeAt(0));
  return bytes.length === width * height * 4 ? new ImageData(bytes, width, height) : null;
}

function paintAppIcon(icon: AppIcon) {
  const image = decode(icon.data, icon.width, icon.height);
  if (!image) return;
  appImage.width = icon.width;
  appImage.height = icon.height;
  appImage.getContext("2d")?.putImageData(image, 0, 0);
  root.dataset.appicon = "on";
}

function paintBackdrop(b: BackdropPayload) {
  // Ảnh đến muộn của một lần mở đã đóng: bỏ qua.
  if (!blurOn || b.seq !== currentSeq || root.dataset.phase === "idle") return;
  const bytes = Uint8ClampedArray.from(atob(b.data), (c) => c.charCodeAt(0));
  if (bytes.length !== b.width * b.height * 4) return;
  backdrop.width = b.width;
  backdrop.height = b.height;
  backdrop.getContext("2d")?.putImageData(new ImageData(bytes, b.width, b.height), 0, 0);
  root.dataset.backdrop = "on";
}

// ---------- Giao diện theo cài đặt ----------

let blurOn = true;

function applyLook(look: MenuLook) {
  root.dataset.tone = look.tone;
  root.dataset.motion = look.animations;
  blurOn = look.blur;
  root.dataset.blur = look.blur ? "on" : "off";
  applyAccent(root, look.accent);
  // Rust phóng cửa sổ theo cùng tỉ lệ; zoom giữ bố cục 520×520 khớp đúng cửa sổ đó.
  root.style.zoom = String(look.size > 0 ? look.size : 1);
}

applyLook({ tone: "dark", accent: "#7C83FF", animations: "system", blur: true, size: 1 });
document.addEventListener("contextmenu", (e) => e.preventDefault());
invoke<MenuLook>("menu_look").then(applyLook).catch(() => {});
listen<MenuLook>("menu:look", (e) => applyLook(e.payload));

listen<OpenPayload>("menu:open", (e) => open(e.payload));
listen<BackdropPayload>("menu:backdrop", (e) => paintBackdrop(e.payload));
listen<AppIcon & { seq: number }>("menu:icon", (e) => {
  if (e.payload.seq === currentSeq && root.dataset.phase !== "idle") paintAppIcon(e.payload);
});
listen<{ dir: Dir }>("menu:dir", (e) => setDir(e.payload.dir));
listen<{ mode: "pick" | "cancel"; dir: Dir }>("menu:close", (e) => close(e.payload.mode, e.payload.dir));
