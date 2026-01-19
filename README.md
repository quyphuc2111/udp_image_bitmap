# Screen Sharing - UDP Multicast

Ứng dụng chia sẻ màn hình sử dụng UDP Multicast cho giảng bài, được xây dựng với Tauri + React.

## Tính năng

- **Server Mode (Giảng viên)**: Chia sẻ màn hình qua UDP Multicast
- **Client Mode (Học viên)**: Nhận và hiển thị màn hình từ server
- Sử dụng UDP Multicast (239.0.0.1:9999) để truyền dữ liệu
- Tự động chia nhỏ frame thành các gói UDP an toàn
- Nén JPEG để tối ưu băng thông
- Tốc độ ~10 FPS

## Cài đặt

### Yêu cầu
- Node.js (v18+)
- Rust (latest stable)
- Cargo

### Cài đặt dependencies

```bash
npm install
```

## Chạy ứng dụng

### Development mode
```bash
npm run tauri dev
```

### Build production
```bash
npm run tauri build
```

## Hướng dẫn sử dụng

### Máy Server (Giảng viên)
1. Mở ứng dụng
2. Chọn "Server (Giảng viên)"
3. Click "Bắt đầu chia sẻ"
4. Màn hình của bạn sẽ được broadcast qua mạng local

### Máy Client (Học viên)
1. Mở ứng dụng
2. Chọn "Client (Học viên)"
3. Click "Kết nối"
4. Màn hình của giảng viên sẽ hiển thị

## Lưu ý

- Cả server và client phải ở cùng mạng local
- Firewall có thể cần được cấu hình để cho phép UDP multicast
- Địa chỉ multicast: 239.0.0.1:9999
- Nếu gặp vấn đề kết nối, kiểm tra firewall và network settings

## Cấu trúc dự án

```
├── src/                    # Frontend React
│   ├── App.tsx            # UI chính
│   └── App.css            # Styles
├── src-tauri/             # Backend Rust
│   └── src/
│       ├── lib.rs         # Entry point & Tauri commands
│       ├── screen_capture.rs  # Screen capture logic
│       ├── udp_server.rs  # UDP multicast server
│       └── udp_client.rs  # UDP multicast client
```

## Công nghệ sử dụng

- **Frontend**: React + TypeScript + Vite
- **Backend**: Rust + Tauri
- **Screen Capture**: scrap crate (hiệu suất cao, cross-platform)
- **Image Processing**: image crate (JPEG compression)
- **Network**: UDP Multicast (std::net)
- **Async Runtime**: Tokio

## Tại sao sử dụng scrap?

- ✅ Hiệu suất cao hơn với direct frame buffer access
- ✅ Hỗ trợ tốt cho Windows, macOS, Linux
- ✅ Low latency capture
- ✅ Control chi tiết hơn về capture process
- ✅ Tích hợp tốt với Rust ecosystem

## License

MIT
