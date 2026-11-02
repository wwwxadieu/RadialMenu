// Trang Hướng dẫn: cách mở menu, các thao tác, cách đổi lệnh, mẹo và câu hỏi thường gặp.
import { card, pageHead, type PageHost } from "./pages";
import { h, icon } from "./dom";
import { RadialPreview } from "./preview";

/** Bản xem trước có con trỏ giả: đi từ tâm ra một hướng, góc đó sáng lên, rồi quay về. */
function gestureDemo(host: PageHost): HTMLElement {
  const app = host.app();
  const sample = app.profiles.find((p) => p.id === "default") ?? app.builtins.find((p) => p.id === "default") ?? app.profiles[0];
  const preview = new RadialPreview({ interactive: false });
  const cursor = h("span", { class: "demo-cursor", "aria-hidden": "true" });
  cursor.innerHTML = '<svg viewBox="0 0 24 24" width="22" height="22"><path d="M5 3l13 8.5-6 1.2 3.4 6.8-2.6 1.3-3.4-6.8L5 18z"/></svg>';
  const box = h("div", { class: "demo" }, preview.el, cursor);
  if (!sample) return box;

  // Thứ tự đi vòng cho dễ thấy: Bắc, Đông, Nam, Tây, rồi các góc chéo.
  const order = [0, 2, 4, 6, 1, 3, 5, 7];
  let step = 0;
  const center = () => {
    cursor.style.left = "50%";
    cursor.style.top = "50%";
    preview.update(sample, null);
  };
  const out = (i: number) => {
    const a = ((-90 + i * 45) * Math.PI) / 180;
    cursor.style.left = `${50 + Math.cos(a) * 41}%`;
    cursor.style.top = `${50 + Math.sin(a) * 41}%`;
    preview.update(sample, i);
  };
  const tick = () => {
    if (!box.isConnected) return;
    if (!document.hidden) {
      const i = order[step++ % order.length];
      out(i);
      window.setTimeout(() => box.isConnected && center(), 1100);
    }
    window.setTimeout(tick, 1700);
  };
  center();
  window.setTimeout(tick, 600);
  return box;
}

function step(n: number, ico: string, title: string, text: string): HTMLElement {
  return h(
    "li",
    { class: "guide-step" },
    h("span", { class: "step-num", "aria-hidden": "true" }, String(n)),
    h("div", { class: "step-body" }, h("h3", null, icon(ico, 16), title), h("p", null, text)),
  );
}

function row(how: (Node | string)[], what: string): HTMLElement {
  return h("tr", null, h("td", null, ...how), h("td", null, what));
}

const kbd = (k: string) => h("kbd", null, k);

function faq(q: string, ...a: (Node | string)[]): HTMLElement {
  return h("details", { class: "faq" }, h("summary", null, q, icon("chevronDown", 16, "ico faq-chev")), h("div", { class: "faq-body" }, ...a));
}

export function guidePage(host: PageHost): HTMLElement {
  const link = (label: string, target: Parameters<PageHost["go"]>[0]) =>
    h("button", { type: "button", class: "link-btn", onclick: () => host.go(target) }, label);

  return h(
    "div",
    { class: "page guide" },
    pageHead("Hướng dẫn", "Cách dùng menu tròn và đổi lệnh theo ý bạn."),

    h(
      "section",
      { class: "card guide-start" },
      h(
        "div",
        { class: "guide-start-text" },
        h("h2", { class: "card-title" }, "Bắt đầu trong 3 bước"),
        h(
          "ol",
          { class: "guide-steps" },
          step(1, "mouse", "Nhấn giữ chuột phải", "Ở bất kỳ đâu, giữ nút phải rồi kéo nhẹ. Menu tròn mở ra ngay tại chỗ nhấn."),
          step(2, "arrowRight", "Kéo về một hướng", "Không cần nhắm chính xác: chỉ cần đúng hướng, góc đó sáng lên."),
          step(3, "check", "Nhả tay", "Lệnh của góc đó chạy ngay. Nhả ở giữa menu nếu đổi ý."),
        ),
        h("p", { class: "hint" }, "Nhấp chuột phải bình thường (không kéo) vẫn ra menu gốc của Windows như cũ."),
      ),
      gestureDemo(host),
    ),

    card(
      "Các thao tác",
      h(
        "table",
        { class: "guide-table" },
        h("thead", null, h("tr", null, h("th", null, "Thao tác"), h("th", null, "Kết quả"))),
        h(
          "tbody",
          null,
          row(["Nhấn giữ chuột phải, kéo nhẹ"], "Mở menu tròn"),
          row(["Kéo về một hướng rồi nhả"], "Chạy lệnh của góc đó"),
          row(["Nhả ở giữa menu, bấm ", kbd("Esc"), " hoặc nút chuột khác"], "Đóng menu, không làm gì"),
          row(["Kéo vụt ra xa khỏi menu"], "Menu tự đóng (giữa menu báo “Đóng”)"),
          row(["Nhấp chuột phải, không kéo"], "Menu chuột phải gốc của Windows"),
          row(["Giữ ", kbd("Shift"), " khi nhấn chuột phải"], "Luôn ra menu gốc"),
        ),
      ),
      h("p", { class: "hint" }, "Ngưỡng kéo, vùng huỷ ở giữa và khoảng cách tự đóng chỉnh được trong ", link("Cử chỉ & khởi động", "behavior"), "."),
    ),

    card(
      "Đổi lệnh trong menu",
      h(
        "p",
        { class: "muted" },
        "Mỗi ứng dụng có một bộ lệnh riêng (profile), hiện ở cột bên trái. Ứng dụng chưa có bộ riêng dùng bộ ",
        h("b", null, "Mặc định"),
        ".",
      ),
      h(
        "ol",
        { class: "guide-list" },
        h("li", null, "Chọn bộ lệnh ở cột bên trái, hoặc bấm ", h("b", null, "Thêm ứng dụng"), " để tạo bộ cho một ứng dụng mới."),
        h(
          "li",
          null,
          "Mở ",
          h("b", null, "Thư viện lệnh"),
          ", kéo một lệnh có sẵn (Sao chép, Dán, Tab mới, Chụp màn hình…) và thả vào một góc trên hình menu.",
        ),
        h("li", null, "Muốn tự đặt phím tắt, chạy chương trình, mở trang web hay gõ một đoạn chữ: bấm vào góc đó rồi sửa ở bảng bên phải."),
        h("li", null, "Bấm ", h("b", null, "Lưu"), " (", kbd("Ctrl"), "+", kbd("S"), "). Menu đổi theo ngay lần mở sau."),
      ),
      h(
        "div",
        { class: "guide-actions" },
        h("button", { type: "button", class: "btn sm", onclick: () => host.go("default") }, icon("grid", 15), "Mở bộ Mặc định"),
        h("button", { type: "button", class: "btn sm", onclick: () => host.go("add-app") }, icon("plus", 15), "Thêm ứng dụng"),
      ),
    ),

    card(
      "Mẹo",
      h(
        "ul",
        { class: "guide-list tips-list" },
        h("li", null, h("b", null, "Nhấn, vụt, nhả: "), "không cần chờ menu hiện hết, cứ vụt về hướng muốn chọn rồi nhả tay."),
        h("li", null, h("b", null, "Đặt lệnh hay dùng ở trên, dưới, trái, phải: "), "bốn hướng này dễ vụt trúng nhất. Dùng vài lần là tay tự nhớ."),
        h("li", null, h("b", null, "Lệnh cần chọn đúng thứ dưới con trỏ "), "(Đổi tên, Xoá…): bật “Click vào chỗ vừa nhấn trước khi chạy” cho góc đó."),
        h("li", null, h("b", null, "Tạm tắt menu tròn: "), "gạt “Bật menu” ở góc dưới bên trái, hoặc chuột phải vào icon Radial Menu dưới khay hệ thống."),
        h("li", null, h("b", null, "Đổi màu, tông sáng tối và kích thước menu "), "trong ", link("Giao diện", "appearance"), "."),
      ),
    ),

    card(
      "Khi có trục trặc",
      faq(
        "Menu không mở trong một ứng dụng",
        h(
          "p",
          null,
          "Radial Menu tự đứng ngoài khi ứng dụng đang chạy toàn màn hình, chạy bằng quyền Admin (Windows không cho gửi phím vào đó), hoặc nằm trong danh sách bỏ qua. Trang ",
          link("Nhật ký", "log"),
          " ghi rõ lý do của từng lần nhấn.",
        ),
      ),
      faq(
        "Lệnh chạy nhưng không đúng chỗ",
        h("p", null, "Bật “Click vào chỗ vừa nhấn trước khi chạy” cho góc đó, để ứng dụng chọn đúng file hay chữ dưới con trỏ trước khi nhận phím tắt."),
      ),
      faq("Góc Dán bị mờ, không chọn được", h("p", null, "Clipboard đang trống. Sao chép một thứ gì đó là góc Dán sáng lại.")),
      faq(
        "Chụp màn hình không thấy menu",
        h("p", null, "Menu được ẩn khỏi ảnh chụp để nền mờ nhanh hơn. Tắt “Ẩn menu khỏi ảnh chụp màn hình” trong ", link("Giao diện", "appearance"), "."),
      ),
      faq(
        "Muốn đưa bộ lệnh về như lúc mới cài",
        h(
          "p",
          null,
          "Trong từng bộ lệnh có sẵn, bấm “Về mặc định” ở phía trên. Muốn đưa tất cả về ban đầu, vào ",
          link("Sao lưu & khôi phục", "backup"),
          " (nên xuất bản sao lưu trước).",
        ),
      ),
    ),
  );
}
