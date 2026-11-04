# Phát triển Radial Menu

Tài liệu cho người muốn build, sửa hoặc đóng góp mã nguồn. Hướng dẫn dùng app nằm trong [README](../README.md).

Radial Menu viết bằng Tauri v2: phần lõi (hook chuột, nhận cử chỉ, chạy lệnh, chụp nền) bằng Rust, giao diện (menu tròn và cửa sổ cài đặt) bằng TypeScript thuần, không framework, chạy trong WebView2.

## Cần cài trước

| Công cụ | Ghi chú |
| --- | --- |
| Rust stable, toolchain MSVC | `rustup default stable-x86_64-pc-windows-msvc`, cần Rust 1.90 trở lên |
| Visual Studio Build Tools | Workload "Desktop development with C++" |
| WebView2 | Có sẵn trên Windows 11 |
| Node.js 20.19 trở lên | Kèm npm |

## Chạy thử

Cách nhanh nhất: bấm đúp `build.cmd`. Script kiểm tra công cụ, cài gói, build bản debug rồi mở app; log nằm trong `logs\`.

Bộ cài để thử trên máy: bấm đúp `build-release.cmd`. Script build bản release, đóng gói `Radial Menu_<phiên bản>_x64-setup.exe` vào `src-tauri\target\release\bundle\nsis\` rồi mở thư mục đó. Lần đầu Tauri tự tải NSIS về. Bộ cài này không có chữ ký cập nhật; bản phát hành chính thức do GitHub Actions build (xem [Phát hành](#phát-hành)).

Hoặc chạy chế độ dev:

```powershell
cd RadialMenu
npm install
npm run tauri dev
```

Lần build đầu mất vài phút. Chạy từ terminal thường, **không** chạy bằng quyền Administrator: app thường không gửi được phím vào app chạy quyền Admin, và điều đó cần được thấy khi kiểm tra.

Chạy test (máy trạng thái cử chỉ, profile, cài đặt, sao lưu, làm mờ nền, tên phím):

```powershell
cd src-tauri
cargo test
```

Kiểm tra kiểu TypeScript và build giao diện:

```powershell
npx tsc --noEmit
npx vite build
```

Icon của app trong `src-tauri/icons` được sinh từ `src/shared/logo.svg`. Sửa logo xong thì chạy `node scripts/make-icons.cjs`.

## Cấu trúc

```text
src-tauri/src/
  main.rs        điểm vào
  lib.rs         khởi động Tauri, chuyển sự kiện cử chỉ, áp dụng cài đặt
  commands.rs    lệnh cho cửa sổ cài đặt và trang menu
  autostart.rs   chạy cùng Windows (khoá Run của người dùng)
  backup.rs      xuất/nhập bản sao lưu (có test)
  appicon.rs     icon thật của app dưới con trỏ, có bộ đệm (có test)
  tray.rs        icon khay hệ thống, mục "Cập nhật lên …" khi có bản mới
  updater.rs     tự cập nhật: hỏi latest.json trên GitHub Releases, tải, kiểm chữ ký, cài
  overlay.rs     cửa sổ menu: hiện tại điểm nhấn, ẩn sau animation đóng
  overlay/win.rs WS_EX_NOACTIVATE, SetWindowPos không kích hoạt, ẩn khỏi ảnh chụp
  profiles.rs    profile JSON: đọc, kiểm tra, lưu, xoá, khớp theo process, 5 bộ mặc định
  settings.rs    settings.json và chuyển đổi giữa các phiên bản cài đặt
  keys.rs        tên phím → mã phím ảo (có test)
  foreground.rs  app dưới con trỏ, quyền Admin, toàn màn hình
  probe.rs       tra app ngoài thread hook, hook chỉ chờ tối đa 25 ms
  actions.rs     chạy hành động trên thread riêng
  actions/win.rs SendInput, click, kích hoạt cửa sổ, ShellExecute
  backdrop.rs    làm mờ ảnh nền (có test); backdrop/win.rs chụp màn hình bằng GDI
  gesture.rs     máy trạng thái cử chỉ (thuần Rust, có test)
  hook.rs        API hook dùng chung, bản giả cho nền tảng khác
  hook/win.rs    WH_MOUSE_LL + WH_KEYBOARD_LL, phát lại right-click
src/menu/        menu tròn (TypeScript thuần + CSS transition)
src/settings/    cửa sổ cài đặt (TypeScript thuần, không framework)
  editor.ts      trang sửa profile: hình menu, thẻ Sửa góc / Thư viện lệnh
  library.ts     thư viện lệnh soạn sẵn, kéo thả bằng sự kiện con trỏ
  colorpicker.ts bộ chọn màu nhấn (ô màu, thanh sắc màu, mã HEX, ống hút)
  guide.ts       trang Hướng dẫn
  pages.ts       các trang chung: Cử chỉ, Giao diện, Nhật ký, Sao lưu
src/shared/      icon, logo (logo.svg), hình học menu, tên phím, kiểu dữ liệu dùng chung
scripts/make-icons.cjs        sinh icon app từ src/shared/logo.svg
scripts/release.cjs           chuẩn bị file phát hành (bộ cài, latest.json, ghi chú) trong GitHub Actions
src-tauri/installer-hooks.nsh bước thêm khi gỡ cài đặt (xoá mục chạy cùng Windows, trừ khi đang cập nhật)
src-tauri/tauri.release.conf.json  bật tạo chữ ký cập nhật, chỉ dùng khi phát hành
.github/workflows/ci.yml      kiểm tra mỗi lần đẩy code (Windows)
.github/workflows/release.yml build và tạo release khi đẩy tag v*
docs/releases/                ghi chú phát hành của từng bản
```

## Cách hook hoạt động

- Callback của hook chỉ cập nhật máy trạng thái rồi đẩy việc sang thread khác. Windows âm thầm gỡ hook nếu callback chạy quá lâu.
- Cú nhấn chuột phải bị giữ lại cho tới khi biết người dùng có kéo hay không. Nếu không kéo, app phát lại right-click tại đúng điểm nhấn, có gắn dấu riêng để hook bỏ qua.
- Sự kiện chuột do phần mềm khác phát ra (điều khiển từ xa, macro) được cho qua nguyên vẹn.
- Bấm nút chuột khác trong lúc giữ chuột phải sẽ trả nút phải lại cho Windows, để không phá thao tác bấm hai nút cùng lúc.
- Kéo ra xa quá khoảng cách tự đóng (tính từ điểm nhấn, nhân theo DPI) thì menu đóng và lần nhả không chạy gì. 250 ms đầu sau khi menu mở không tính, dựa trên thời điểm của chính sự kiện chuột, để cú vụt nhanh vẫn chọn được hướng.
- Menu con: khi menu mở, app báo cho hook góc nào có menu con (`hook::set_submenus`). Kéo qua mép đĩa ở góc đó thì máy trạng thái phát `Effect::Drill` và vòng lệnh đổi sang menu con ngay tại chỗ (cùng tâm, chọn theo góc như vòng chính). Nhả chuột ở góc đó thì vào chế độ bấm chọn (`Phase::Sticky`): di chuột đổi hướng, cú bấm nút bất kỳ chọn lệnh và bị nuốt (cả lần nhả), bấm hẳn ra ngoài menu thì đóng menu và cú bấm đi tiếp tới app, bỏ không quá 15 giây thì tự huỷ.

## Nền kính mờ

Lúc menu mở, app chụp vùng màn hình dưới đĩa menu bằng GDI, thu nhỏ còn khoảng 128 px, làm mờ (3 lần box blur ≈ `blur(20px)`) và tăng bão hoà 170% trong Rust, rồi vẽ lên canvas dưới lớp màu kính. Khi bật "Ẩn menu khỏi ảnh chụp màn hình", cửa sổ menu được đánh dấu `WDA_EXCLUDEFROMCAPTURE` nên việc chụp chạy song song sau khi menu đã hiện; tắt đi thì menu chụp được nhưng nền phải chụp trước lúc menu hiện.

## Cấu hình

Thư mục: `%APPDATA%\com.mrbeohp.radialmenu\`

```text
settings.json        cử chỉ, giao diện, chạy cùng Windows (sửa trong cửa sổ cài đặt)
profiles/
  browser.json       chrome, msedge, firefox, brave, opera, vivaldi
  code.json          Code, Cursor, Windsurf, VSCodium
  default.json       mọi app khác
  desktop.json       desktop (explorer.exe, lớp cửa sổ Progman/WorkerW)
  explorer.json      cửa sổ File Explorer
```

Mỗi góc trong profile:

```json
{ "dir": "SW", "label": "Mở lại tab", "icon": "undo",
  "action": { "type": "hotkey", "keys": ["Ctrl", "Shift", "T"] } }
```

| `type` | Trường | Ví dụ |
| --- | --- | --- |
| `hotkey` | `keys` | `["Alt", "Left"]`, `["Win", "Shift", "S"]`, `["F2"]` |
| `command` | `cmd`, `args`, `cwd` | `"cmd": "notepad.exe"`; hỗ trợ `%USERPROFILE%` |
| `url` | `url` | `"https://github.com"`, hoặc đường dẫn file/thư mục |
| `text` | `text` | `"Trân trọng,\nMrBeoHP"` |
| `sequence` | `steps` | danh sách các hành động trên, chạy lần lượt |
| `wait` | `ms` | chỉ dùng trong `sequence`, tối đa 5000 |
| `settings` | (không có) | mở cửa sổ cài đặt; không dùng trong `sequence` |
| `submenu` | `slots` | danh sách góc như của profile, không lồng thêm `submenu`; không dùng trong `sequence` |

Tuỳ chọn thêm cho mỗi góc:

- `"clickFirst": true`: click vào điểm nhấn trước khi chạy, để chọn đúng thứ dưới con trỏ.
- `"when": "clipboard"`: góc chỉ bật khi clipboard có dữ liệu (`"always"` để luôn bật). Không ghi thì góc gửi Ctrl+V hoặc Shift+Insert tự hiểu là `"clipboard"`.

Profile khớp theo `"match": { "process": [...], "class": [...] }`. Khi nhiều profile cùng khớp, profile có nhiều điều kiện hơn thắng, nên Desktop (process + lớp cửa sổ) thắng Explorer (chỉ process). Trong `cmd`, `args`, `cwd`, `url` có thể dùng biến môi trường và `%DESKTOP%`, `%DOCUMENTS%`, `%DOWNLOADS%`.

Bản mới có thêm profile mặc định nào thì app tự ghi file đó vào thư mục, không đụng tới file bạn đã sửa. Sửa tay xong thì bấm "Nạp lại" trong trang Sao lưu & khôi phục hoặc ở khay hệ thống.

## Phát hành

Bản phát hành do GitHub Actions build (`.github/workflows/release.yml`), có chữ ký cho tính năng tự cập nhật:

1. Nâng phiên bản ở `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` (và chạy `cargo check` để cập nhật `Cargo.lock`).
2. Viết ghi chú ở `docs/releases/vX.Y.Z.md`. Ghi chú này hiện trên trang release và trong hộp thoại cập nhật của app (bỏ ảnh).
3. Đẩy code lên `main`, rồi `git tag vX.Y.Z && git push origin vX.Y.Z`.

Workflow chạy test, build bộ cài với `--config src-tauri/tauri.release.conf.json` để tạo file chữ ký `.sig`, rồi `scripts/release.cjs` chuẩn bị `RadialMenu_X.Y.Z_x64-setup.exe`, `latest.json` và ghi chú (kèm SHA-256) cho `gh release create`.

App đã cài hỏi `https://github.com/wwwxadieu/RadialMenu/releases/latest/download/latest.json`, so phiên bản, tải bộ cài và chỉ cài khi chữ ký khớp khoá công khai trong `tauri.conf.json` (`plugins.updater.pubkey`). Khoá bí mật nằm trong secret `TAURI_SIGNING_PRIVATE_KEY` của repo; mất khoá thì các bản đã cài không nhận được cập nhật nữa.

## Kiểm tra thủ công trước khi phát hành

1. Mở app: cửa sổ cài đặt hiện ra; sửa tên một góc, bấm Lưu, menu đổi theo ngay.
2. Thêm ứng dụng Notepad từ danh sách app đang mở, đặt Ctrl+S cho một góc, lưu, thử trong Notepad.
3. Xoá tên một góc rồi Lưu: thanh lưu báo lỗi đúng góc, bản xem trước nhảy tới góc đó.
4. Giao diện: đổi sang tông Sáng, màu cam và kích thước 120%, menu tròn đổi theo ở lần mở sau.
5. Clipboard trống (vừa khởi động máy, hoặc Win+V › Xoá tất cả): góc Dán trong Explorer mờ, không chạy; copy một file thì sáng lại.
6. Giữa menu hiện icon thật của Notepad, Chrome…; trên Desktop vẫn là icon màn hình.
7. Mở menu rồi kéo vụt ra xa: menu đóng, nhả chuột không chạy gì. Nhấn, vụt, nhả thật nhanh vẫn chọn được góc.
8. Bật Chạy cùng Windows (ở bản cài đặt), đăng xuất rồi vào lại: app nằm dưới khay, không mở cửa sổ.
9. Xuất bản sao lưu, khôi phục profile mặc định, nhập lại bản sao lưu: các thay đổi quay về.
10. Cài bằng `-setup.exe`: hỏi quyền Admin, cài vào Program Files, có trong Start menu; gỡ cài đặt sạch.
11. Menu con: đặt một góc là Menu con có 3 lệnh. Kéo qua mép ở góc đó: vòng lệnh đổi sang menu con, kéo tiếp rồi nhả là chạy. Nhả ở góc đó: menu con chờ bấm, bấm trái vào một lệnh là chạy, bấm ra ngoài thì đóng và cú bấm vẫn tới app bên dưới.
12. Cập nhật: cài bản cũ hơn từ trang Releases, mở cài đặt, bấm "Kiểm tra bản mới" rồi "Cập nhật ngay": hỏi quyền Admin, cài xong app tự mở lại, "Chạy cùng Windows" vẫn giữ.
