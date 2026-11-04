// Gọi lõi Rust (commands.rs).
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, AppState, Profile, Settings, UpdateInfo } from "../shared/types";

export const api = {
  state: () => invoke<AppState>("get_state"),
  saveProfile: (profile: Profile, originalId: string | null) => invoke<Profile>("save_profile", { profile, originalId }),
  deleteProfile: (id: string) => invoke<void>("delete_profile", { id }),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  restoreDefaults: () => invoke<number>("restore_defaults"),
  exportBackup: () => invoke<string>("export_backup"),
  importBackup: (text: string) => invoke<{ profiles: string[]; settings: boolean }>("import_backup", { text }),
  listApps: () => invoke<AppInfo[]>("list_apps"),
  setPaused: (paused: boolean) => invoke<boolean>("set_paused", { paused }),
  reload: () => invoke<void>("reload_config"),
  openConfigDir: () => invoke<void>("open_config_dir"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  installUpdate: () => invoke<void>("install_update"),
};

/** Lỗi từ Rust là chuỗi; lỗi khác thì lấy message. */
export const errorText = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : String(e));
