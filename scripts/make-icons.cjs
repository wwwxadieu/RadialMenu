// Sinh icon của app (src-tauri/icons) từ đúng file logo dùng trong cửa sổ cài đặt
// (src/shared/logo.svg), để logo và icon luôn giống nhau.
// Chạy: node scripts/make-icons.cjs   (cần Playwright + Python có Pillow)
const { chromium } = require("playwright");
const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const root = path.join(__dirname, "..");
const svg = fs.readFileSync(path.join(root, "src/shared/logo.svg"), "utf8");
const out = path.join(root, "src-tauri/icons");
const tmp = fs.mkdtempSync(path.join(require("os").tmpdir(), "radial-icons-"));

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const sizes = [16, 24, 32, 48, 64, 128, 256, 512];
  for (const size of sizes) {
    await page.setViewportSize({ width: size, height: size });
    await page.setContent(`<html><body style="margin:0;background:transparent">${svg.replace("<svg ", `<svg width="${size}" height="${size}" `)}</body></html>`);
    await page.screenshot({ path: path.join(tmp, `${size}.png`), omitBackground: true });
  }
  await browser.close();

  fs.copyFileSync(path.join(tmp, "32.png"), path.join(out, "32x32.png"));
  fs.copyFileSync(path.join(tmp, "128.png"), path.join(out, "128x128.png"));
  fs.copyFileSync(path.join(tmp, "256.png"), path.join(out, "128x128@2x.png"));
  fs.copyFileSync(path.join(tmp, "512.png"), path.join(out, "icon.png"));
  // File .ico nhiều cỡ, mỗi cỡ vẽ riêng cho nét.
  execFileSync("python3", ["-c", `
from PIL import Image
import sys
tmp = sys.argv[1]
sizes = [16, 24, 32, 48, 64, 128, 256]
imgs = [Image.open(f"{tmp}/{s}.png").convert("RGBA") for s in sizes]
imgs[-1].save(sys.argv[2], format="ICO", sizes=[(s, s) for s in sizes], append_images=imgs[:-1])
`, tmp, path.join(out, "icon.ico")]);
  console.log("Đã ghi icon vào", out);
})();
