// Tên phím giống `keys.rs`: hiển thị đẹp, tách/ghép tổ hợp, đọc phím từ bàn phím.

export const MODIFIERS = ["Ctrl", "Shift", "Alt", "Win"] as const;
export type Modifier = (typeof MODIFIERS)[number];

const MOD_ALIASES: Record<string, Modifier> = {
  ctrl: "Ctrl",
  control: "Ctrl",
  shift: "Shift",
  alt: "Alt",
  win: "Win",
  windows: "Win",
  meta: "Win",
  super: "Win",
  cmd: "Win",
};

export function asModifier(name: string): Modifier | null {
  return MOD_ALIASES[name.trim().toLowerCase()] ?? null;
}

const PRETTY: Record<string, string> = {
  left: "←",
  right: "→",
  up: "↑",
  down: "↓",
  enter: "Enter",
  return: "Enter",
  esc: "Esc",
  escape: "Esc",
  delete: "Delete",
  del: "Delete",
  space: "Space",
  spacebar: "Space",
  tab: "Tab",
  backspace: "Backspace",
  pageup: "PageUp",
  pagedown: "PageDown",
  home: "Home",
  end: "End",
  insert: "Insert",
  apps: "Menu",
  printscreen: "PrtSc",
  volumeup: "Âm lượng +",
  volumedown: "Âm lượng −",
  volumemute: "Tắt tiếng",
  mediaplay: "Phát/Dừng",
  medianext: "Bài sau",
  mediaprev: "Bài trước",
};

export function prettyKey(name: string): string {
  const raw = name.trim();
  const lower = raw.toLowerCase();
  const mod = asModifier(raw);
  if (mod) return mod;
  if (PRETTY[lower]) return PRETTY[lower];
  if (raw.length === 1) return raw.toUpperCase();
  if (/^f\d{1,2}$/.test(lower)) return raw.toUpperCase();
  return raw;
}

export const prettyChord = (keys: string[]) => keys.map(prettyKey).join("+");

export interface Chord {
  mods: Set<Modifier>;
  /** Phím chính (không phải phím bổ trợ). */
  key: string | null;
}

export function splitChord(keys: string[]): Chord {
  const mods = new Set<Modifier>();
  let key: string | null = null;
  for (const k of keys) {
    const m = asModifier(k);
    if (m) mods.add(m);
    else key = k;
  }
  return { mods, key };
}

export function joinChord({ mods, key }: Chord): string[] {
  const out: string[] = MODIFIERS.filter((m) => mods.has(m));
  if (key) out.push(key);
  return out;
}

/** Phím đặc biệt cho ô chọn (những phím khó bấm để ghi lại). */
export const SPECIAL_KEYS: ReadonlyArray<readonly [string, string]> = [
  ["Enter", "Enter"],
  ["Esc", "Esc"],
  ["Tab", "Tab"],
  ["Space", "Space"],
  ["Backspace", "Backspace"],
  ["Delete", "Delete"],
  ["Insert", "Insert"],
  ["Home", "Home"],
  ["End", "End"],
  ["PageUp", "PageUp"],
  ["PageDown", "PageDown"],
  ["Up", "↑ Lên"],
  ["Down", "↓ Xuống"],
  ["Left", "← Trái"],
  ["Right", "→ Phải"],
  ...Array.from({ length: 12 }, (_, i) => [`F${i + 1}`, `F${i + 1}`] as const),
  ["PrintScreen", "PrtSc"],
  ["Apps", "Phím Menu"],
  ["VolumeUp", "Âm lượng +"],
  ["VolumeDown", "Âm lượng −"],
  ["VolumeMute", "Tắt tiếng"],
  ["MediaPlay", "Phát/Dừng"],
  ["MediaNext", "Bài sau"],
  ["MediaPrev", "Bài trước"],
];

const CODE_KEYS: Record<string, string> = {
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ArrowDown: "Down",
  Enter: "Enter",
  NumpadEnter: "Enter",
  Tab: "Tab",
  Escape: "Esc",
  Space: "Space",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  PrintScreen: "PrintScreen",
  ContextMenu: "Apps",
  CapsLock: "CapsLock",
  NumLock: "NumLock",
  ScrollLock: "ScrollLock",
  Pause: "Pause",
  AudioVolumeUp: "VolumeUp",
  AudioVolumeDown: "VolumeDown",
  AudioVolumeMute: "VolumeMute",
  MediaPlayPause: "MediaPlay",
  MediaTrackNext: "MediaNext",
  MediaTrackPrevious: "MediaPrev",
};

/** Phím chính từ một sự kiện bàn phím (theo vị trí phím, bố cục US); `null` nếu là phím bổ trợ. */
export function keyFromCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^Numpad\d$/.test(code)) return code.slice(6);
  if (/^F\d{1,2}$/.test(code)) return code;
  return CODE_KEYS[code] ?? null;
}
