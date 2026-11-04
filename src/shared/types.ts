// Kiểu dữ liệu trao đổi với lõi Rust (khớp serde của profiles.rs, settings.rs, commands.rs).

export type DirId = "N" | "NE" | "E" | "SE" | "S" | "SW" | "W" | "NW";

export type Action =
  | { type: "hotkey"; keys: string[] }
  | { type: "command"; cmd: string; args?: string[]; cwd?: string | null }
  | { type: "url"; url: string }
  | { type: "text"; text: string }
  | { type: "sequence"; steps: Action[] }
  | { type: "wait"; ms: number }
  | { type: "settings" }
  | { type: "openWith"; app?: string }
  | { type: "submenu"; slots: Slot[] };

export type ActionType = Action["type"];

export interface Slot {
  dir: DirId;
  label: string;
  icon: string;
  action: Action;
  clickFirst?: boolean;
  /** Bỏ trống = tự đoán (góc Dán cần clipboard có dữ liệu). */
  when?: "always" | "clipboard";
}

export interface Profile {
  id: string;
  name: string;
  icon: string;
  match: { process: string[]; class?: string[] };
  enabled: boolean;
  slots: Slot[];
}

export interface Settings {
  thresholdPx: number;
  cancelRadiusPx: number;
  closeWhenFar: boolean;
  closeDistancePx: number;
  animations: "system" | "on" | "off";
  disableWhenFullscreen: boolean;
  exclude: string[];
  tone: "dark" | "light";
  accent: string;
  blur: boolean;
  hideFromCapture: boolean;
  menuScale: number;
  launchAtLogin: boolean;
  /** Tự hỏi GitHub có bản mới không. */
  checkUpdates: boolean;
  profilesVersion: number;
}

/** Bản mới tìm thấy trên GitHub Releases. */
export interface UpdateInfo {
  version: string;
  current: string;
  notes: string;
  date: string | null;
}

export interface AppState {
  starting: boolean;
  paused: boolean;
  hooked: boolean;
  hookError: string | null;
  settings: Settings;
  defaults: Settings;
  profiles: Profile[];
  builtins: Profile[];
  accents: string[];
  configDir: string;
  loadErrors: string[];
  recent: string[];
  version: string;
  update: UpdateInfo | null;
}

export interface AppInfo {
  process: string;
  title: string;
}

export interface MenuLook {
  tone: "dark" | "light";
  accent: string;
  animations: "system" | "on" | "off";
  blur: boolean;
  /** Tỉ lệ kích thước menu, 1 = mặc định. */
  size: number;
}

export type Dir = number | null;

export type GestureEvent =
  | { kind: "open"; x: number; y: number; scale: number; dir: Dir; process: string | null; profile: string }
  | { kind: "dir"; dir: Dir }
  | { kind: "cancel" }
  | { kind: "away" }
  | { kind: "drill"; dir: number; label: string; sticky: boolean }
  | { kind: "empty"; dir: number }
  | { kind: "unavailable"; dir: number; label: string; reason: string }
  | { kind: "run"; dir: number; label: string; detail: string; process: string | null }
  | { kind: "error"; label: string; message: string }
  | { kind: "native" }
  | { kind: "handover" }
  | { kind: "skip"; process: string; reason: string }
  | { kind: "latency"; ms: number }
  | { kind: "loaded"; profiles: number; errors: string[] };
