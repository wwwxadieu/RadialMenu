// Chuẩn bị file phát hành sau khi `tauri build` xong (chạy trong GitHub Actions):
//   node scripts/release.cjs v0.3.0
// Tạo thư mục release/ gồm:
//   RadialMenu_<ver>_x64-setup.exe  bộ cài (đổi tên cho khỏi dấu cách)
//   latest.json                     thông tin cho tính năng tự cập nhật trong app
//   body.md                         ghi chú phát hành: docs/releases/<tag>.md + cách cài + SHA-256
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");

const root = path.join(__dirname, "..");
const repo = process.env.GITHUB_REPOSITORY || "wwwxadieu/RadialMenu";
const tag = process.argv[2] || process.env.GITHUB_REF_NAME;
const fail = (msg) => {
  console.error(`::error::${msg}`);
  process.exit(1);
};

if (!tag) fail("Thiếu tên tag, ví dụ: node scripts/release.cjs v0.3.0");
const conf = JSON.parse(fs.readFileSync(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"));
const version = conf.version;
if (tag !== `v${version}`) fail(`Tag ${tag} không khớp phiên bản ${version} trong tauri.conf.json`);

const nsis = path.join(root, "src-tauri", "target", "release", "bundle", "nsis");
const built = path.join(nsis, `${conf.productName}_${version}_x64-setup.exe`);
if (!fs.existsSync(built)) fail(`Không thấy bộ cài ${built}`);
if (!fs.existsSync(`${built}.sig`)) fail("Không thấy file chữ ký .sig: thiếu secret TAURI_SIGNING_PRIVATE_KEY?");

const notesFile = path.join(root, "docs", "releases", `${tag}.md`);
if (!fs.existsSync(notesFile)) fail(`Thiếu ghi chú phát hành ${path.relative(root, notesFile)}`);
const notes = fs.readFileSync(notesFile, "utf8").trim();

const out = path.join(root, "release");
fs.mkdirSync(out, { recursive: true });
const exeName = `RadialMenu_${version}_x64-setup.exe`;
const exe = path.join(out, exeName);
fs.copyFileSync(built, exe);
const sha = crypto.createHash("sha256").update(fs.readFileSync(exe)).digest("hex");

// Ghi chú trong app: bỏ ảnh (thẻ <img>), giữ chữ.
const appNotes = notes
  .split(/\r?\n/)
  .filter((l) => !l.trim().startsWith("<"))
  .join("\n")
  .replace(/\n{3,}/g, "\n\n")
  .trim();

const latest = {
  version,
  notes: appNotes,
  pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
  platforms: {
    "windows-x86_64": {
      signature: fs.readFileSync(`${built}.sig`, "utf8").trim(),
      url: `https://github.com/${repo}/releases/download/${tag}/${exeName}`,
    },
  },
};
fs.writeFileSync(path.join(out, "latest.json"), JSON.stringify(latest, null, 2) + "\n");

const body = `${notes}

## Cài đặt

Từ bản 0.3.0, Radial Menu tự báo khi có bản mới: bấm **Cập nhật** trong app là xong.

Cài lần đầu, hoặc nâng từ bản 0.2.x trở về trước: tải \`${exeName}\` ở mục **Assets** bên dưới và chạy. Cài đặt và bộ lệnh của bạn được giữ nguyên.

Bộ cài chưa được ký số nên Windows SmartScreen có thể cảnh báo: chọn **More info › Run anyway**. Hướng dẫn đầy đủ có trong [README](https://github.com/${repo}#readme).

**SHA-256** của bộ cài:

\`\`\`
${sha}
\`\`\`
`;
fs.writeFileSync(path.join(out, "body.md"), body);

console.log(`Bộ cài: ${exeName}`);
console.log(`SHA-256: ${sha}`);
