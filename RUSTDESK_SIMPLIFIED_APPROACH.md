# 🚀 RustDesk-Inspired Simplified Approach for LAN

## 📋 Phân tích code RustDesk

### ✅ Ưu điểm cần học:
1. **Multi-platform screen capture** với fallback mechanism
2. **Hardware acceleration** khi có thể (DXGI, VRAM, HwRAM)
3. **Adaptive quality** based on network conditions
4. **Frame pacing** với VideoFrameController
5. **Proper error recovery** và switch mechanism

### ❌ Phần không cần (quá phức tạp cho LAN):
1. QoS adaptive bitrate (LAN ổn định)
2. Multiple codec support (chỉ cần 1-2 codec)
3. Recording feature
4. Privacy mode
5. UAC elevation check
6. Camera support

---

## 🎯 Architecture đề xuất cho LAN

```rust
// Simplified flow:
Display → Capturer → [Optional Encoder] → UDP Multicast → Client
```

### 1️⃣ **Capturer Layer** (Platform-specific)

```rust
// src-tauri/src/capturer_manager.rs
pub trait SimpleCapturer {
    fn capture_frame(&mut self) -> Result<FrameData, String>;
    fn dimensions(&self) -> (usize, usize);
}

pub enum FrameData {
    Raw(Vec<u8>),      // RGBA/BGRA raw pixels
    Compressed(Vec<u8>) // Pre-compressed (JPEG/PNG)
}

// Windows: Use DXGI first, fallback to GDI
#[cfg(windows)]
pub struct WindowsCapturer {
    dxgi: Option<DxgiCapturer>,
    gdi: Option<GdiCapturer>,
}

// macOS: CoreGraphics
#[cfg(target_os = "macos")]
pub struct MacCapturer {
    display: CGDisplay,
}

// Linux: X11/Wayland via scrap
#[cfg(target_os = "linux")]
pub struct LinuxCapturer {
    scrap: scrap::Capturer,
}
```

### 2️⃣ **Compression Strategy** (LAN-optimized)

```rust
// For LAN: Prioritize SPEED over size
pub enum CompressionMode {
    None,              // Raw RGBA (fastest, ~100MB/s for 1080p@30fps)
    FastJPEG(u8),      // Quality 70-85 (good speed/size balance)
    Hardware(Codec),   // H264 if available
}

// LAN bandwidth calculation:
// 1920x1080 @ 30fps
// - Raw: ~240 MB/s (too much for 1Gbps)
// - JPEG 70%: ~15-30 MB/s (OK for 1Gbps)
// - H264: ~5-10 MB/s (best, if HW available)
```

### 3️⃣ **Frame Pacing** (Simplified from RustDesk)

```rust
pub struct FramePacer {
    target_fps: u32,
    last_frame: Instant,
}

impl FramePacer {
    pub fn should_capture(&mut self) -> bool {
        let spf = Duration::from_millis(1000 / self.target_fps as u64);
        if self.last_frame.elapsed() >= spf {
            self.last_frame = Instant::now();
            true
        } else {
            false
        }
    }
    
    pub fn sleep_until_next(&self) {
        let spf = Duration::from_millis(1000 / self.target_fps as u64);
        if let Some(sleep_time) = spf.checked_sub(self.last_frame.elapsed()) {
            std::thread::sleep(sleep_time);
        }
    }
}
```

---

## 📦 Dependencies cần thêm

```toml
# Cargo.toml additions
[dependencies]
# For hardware encoding (optional, Windows only)
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Graphics_Gdi",
    "Win32_Graphics_Direct3D11",
    "Graphics_Capture",
] }

# JPEG compression (faster than image crate for realtime)
turbojpeg = { version = "1.0", optional = true }

# Or stick with image crate
image = { version = "0.25", default-features = false, features = ["jpeg"] }
```

---

## 🔧 Implementation Plan

### Phase 1: Enhanced Capturer (Week 1)
- [ ] Implement DXGI capturer for Windows
- [ ] Add GDI fallback
- [ ] Keep scrap for macOS/Linux
- [ ] Benchmark capture performance

### Phase 2: Smart Compression (Week 2)
- [ ] Implement fast JPEG compression
- [ ] Test quality levels (50, 70, 85)
- [ ] Measure compression time vs quality
- [ ] Add hardware H264 if time permits

### Phase 3: Optimized UDP (Week 3)
- [ ] Implement RustDesk-style chunking
- [ ] Add frame sequence numbers
- [ ] Duplicate critical packets (first/last chunk)
- [ ] Client-side frame assembly with timeout

### Phase 4: Polish (Week 4)
- [ ] FPS counter and stats
- [ ] Auto quality adjustment based on packet loss
- [ ] Mouse cursor capture (Windows)
- [ ] Multi-monitor support

---

## 🎨 Simplified Screen Capture (RustDesk-style)

### Key Learnings from RustDesk code:

```rust
// 1. Frame acquisition with retry
let frame = loop {
    match capturer.frame(timeout) {
        Ok(frame) if frame.valid() => break frame,
        Err(e) if e.kind() == WouldBlock => {
            // Wait for frame
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        Err(e) => return Err(e),
    }
};

// 2. Pixel format conversion
let rgba = match frame {
    Frame::PixelBuffer(buf) => {
        // Convert BGRA → RGBA if needed
        convert_pixel_format(buf)
    }
    Frame::Texture(tex) => {
        // GPU texture, need special handling
        texture_to_rgba(tex)
    }
};

// 3. Compression with error handling
match compress_jpeg(&rgba, quality) {
    Ok(data) => Ok(data),
    Err(e) => {
        // Retry with lower quality
        compress_jpeg(&rgba, quality - 10)?
    }
}
```

---

## 📊 Expected Performance (LAN)

| Resolution | FPS | Compression | Bandwidth | CPU Usage |
|------------|-----|-------------|-----------|-----------|
| 1920x1080  | 30  | JPEG 70%    | ~20 MB/s  | ~15%      |
| 1920x1080  | 30  | H264 (HW)   | ~8 MB/s   | ~5%       |
| 1920x1080  | 60  | JPEG 70%    | ~40 MB/s  | ~25%      |
| 1280x720   | 30  | JPEG 70%    | ~10 MB/s  | ~8%       |

**LAN Requirements:**
- ✅ 100 Mbps: OK for 720p@30fps
- ✅ 1 Gbps: OK for 1080p@60fps
- ✅✅ 10 Gbps: Overkill, can use lower compression

---

## 🚀 Quick Wins from RustDesk approach

### 1. DXGI Fallback Pattern
```rust
// Try hardware first, fallback to software
let mut capturer = try_dxgi().or_else(|_| try_gdi())?;
```

### 2. Frame Validation
```rust
// Don't send empty/corrupt frames
if frame.len() < 100 || !is_valid_jpeg(frame) {
    return Err("Invalid frame");
}
```

### 3. Chunking Strategy
```rust
// Send first and last chunk twice (JPEG headers/footers critical)
send_chunk(0, data); // First chunk
send_chunks(1..n-1, data); // Middle chunks
send_chunk(n-1, data); // Last chunk
// Resend critical chunks
send_chunk(0, data);
send_chunk(n-1, data);
```

### 4. Client Assembly
```rust
// Accept partial frames (95%+ complete) on LAN
if completion_ratio >= 0.95 {
    try_decode_partial_jpeg(chunks)
} else {
    wait_for_next_frame()
}
```

---

## 🎯 Implementation Priority

### Must Have (Core)
1. ✅ Fast screen capture (DXGI/scrap)
2. ✅ JPEG compression (quality 70)
3. ✅ UDP multicast with chunking
4. ✅ Frame assembly on client
5. ✅ Error recovery (keep last frame)

### Nice to Have
1. Hardware H264 encoding
2. Mouse cursor capture
3. Multi-monitor
4. Auto quality adjustment
5. Stats overlay

### Maybe Later
1. Audio capture
2. Multiple compression options
3. Client-side zoom
4. Recording feature

---

## 💡 Key Takeaways từ RustDesk

1. **Always have fallback**: DXGI → GDI → scrap
2. **Validate everything**: Frame size, JPEG headers, chunk count
3. **Frame pacing matters**: Consistent FPS > High FPS
4. **UDP needs redundancy**: Resend critical packets
5. **Keep last valid frame**: Better than black screen
6. **Monitor performance**: FPS, bandwidth, packet loss
7. **Switch on error**: Don't crash, switch to fallback

---

## 📝 Next Steps

1. Read RustDesk's DXGI implementation: `libs/scrap/src/common/dxgi.rs`
2. Study their chunking: `src/server/video_service.rs:handle_one_frame`
3. Check their encoder configs: `get_encoder_config()`
4. Learn frame validation: `Frame::valid()`, `get_rgba_from_pixelbuf()`

Sau đó implement từng phần một cách tối giản cho LAN!
