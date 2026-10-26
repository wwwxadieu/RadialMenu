// Bản release không mở thêm cửa sổ console trên Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    radial_menu_lib::run()
}
