# Quản lý linh kiện (Tauri 2 + Rust)

Ứng dụng desktop Windows để quản lý linh kiện điện tử và theo dõi tồn kho.
Giao diện là HTML/CSS/JS thuần, phần xử lý dữ liệu viết bằng Rust, đóng gói bằng Tauri 2
(dùng WebView2 có sẵn trên Windows 10/11 nên file exe nhỏ).

## Tính năng

- Bảng linh kiện với các cột: Part number, Name, Footprint, Pitch, Quantity, Descriptions,
  Manufacturer, Manufacturer Part Number, Product Category, Vị trí.
- **Lấy** linh kiện: bắt buộc nhập tên người lấy; ngày, thứ và giờ chạy theo đồng hồ và được
  chốt đúng lúc bấm "Xác nhận lấy"; hiện ngay số còn lại. Không cho lấy quá số tồn.
- **Thêm** (nhập kho): hiện tổng tồn sau khi thêm.
- Thêm mới / sửa / xóa linh kiện; chọn từng dòng hoặc chọn tất cả rồi "Xóa đã chọn".
  Mọi hộp thoại xác nhận do app tự hiện (không dùng alert/confirm của trình duyệt).
- Tìm kiếm, lọc theo danh mục / vị trí / hết hàng, sắp xếp theo cột, bảng cuộn bên trong khung.
- **Lịch sử** mọi thao tác: ngày, thứ, giờ, loại thao tác, số lượng +/-, tồn sau, người, ghi chú.
- **Import Excel/CSV** (.xlsx, .xls, .csv): tự tìm dòng tiêu đề, nhận tên cột tiếng Việt lẫn tiếng Anh
  (Số lượng / Qty / Quantity…), xem trước rồi mới ghi. Với part number đã có: cộng dồn, ghi đè
  hoặc giữ nguyên số lượng; các ô trống được điền từ file.
- **Xuất Excel**: 2 sheet "Ton kho" và "Lich su" (có cột Ngày / Thứ / Giờ).
- File Excel mẫu để import; **Sao lưu** / **Khôi phục** toàn bộ dữ liệu ra file .json.

## Dữ liệu lưu ở đâu

Dữ liệu nằm trong một file JSON trên máy, không gửi lên mạng:

```
%APPDATA%\vn.sysone.component-inventory\inventory.json
```

Đường dẫn chính xác hiện ở thanh trạng thái dưới cùng của app. Mỗi lần ghi, bản trước đó được
giữ ở `inventory.json.bak`. Vẫn nên dùng "Sao lưu" định kỳ ra ổ khác.

## Build file exe bằng GitHub Actions (khuyến nghị)

1. Đẩy mã nguồn lên một repo GitHub (nhánh `main`).
2. Workflow `.github/workflows/build-windows.yml` tự chạy khi push, hoặc chạy tay ở tab
   **Actions → Build Windows exe → Run workflow**.
3. Tải kết quả ở phần **Artifacts** của lần chạy: `QuanLyLinhKien-windows-x64.zip` gồm
   - `QuanLyLinhKien_0.1.0_x64-setup.exe`: bộ cài (cài cho user hiện tại, không cần quyền admin).
   - `QuanLyLinhKien-portable.exe`: chạy trực tiếp không cần cài (máy cần có WebView2, Windows 10/11 đều có sẵn).
4. Gắn tag dạng `v0.1.0` và push tag thì workflow còn tạo GitHub Release kèm hai file trên.

Lần build đầu mất khoảng 10–15 phút (biên dịch Rust); các lần sau nhanh hơn nhờ cache.

## Build trên máy Windows (tuỳ chọn)

Cần: [Rust](https://rustup.rs), Visual Studio Build Tools (workload "Desktop development with C++"),
Node.js 18+.

```powershell
npm install
npm run build          # exe + bộ cài trong target\release\ và target\release\bundle\nsis\
npm run dev            # chạy thử có hot reload
```

## Cấu trúc mã

```
src/                    Giao diện (index.html, style.css, app.js) - không cần bundler
crates/inventory-core/  Lõi Rust thuần: model, lưu JSON, import xlsx/csv, xuất xlsx (có test)
src-tauri/              Vỏ Tauri: các lệnh IPC, hộp thoại chọn file, cấu hình, icon
```

Test lõi chạy được trên mọi máy có Rust, không cần WebView:

```bash
cargo test -p inventory-core
```
