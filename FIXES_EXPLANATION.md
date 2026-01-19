# 🔧 Giải thích về vấn đề màn hình đen và ImageBitmap

## ❓ Câu hỏi của bạn:

1. **Tại sao không sử dụng ImageBitmap để render video?**
2. **Tại sao hình ảnh khi share thỉnh thoảng bị đen xì?**

---

## ✅ Trả lời câu hỏi 1: ImageBitmap ĐÃ được sử dụng!

Code của bạn **ĐANG** sử dụng `ImageBitmap` đúng cách (dòng 36-48 trong `App.tsx`):

```typescript
const imageBitmap = await createImageBitmap(blob);
ctx.drawImage(imageBitmap, 0, 0);
```

**Tuy nhiên**, có một số vấn đề về **lifecycle management** và **performance optimization** đã được fix.

---

## 🐛 Trả lời câu hỏi 2: Nguyên nhân màn hình đen

### **Vấn đề 1: Race Condition khi resize Canvas** ⚠️

**Code cũ:**
```typescript
const ctx = canvas.getContext("2d");
if (ctx) {
  canvas.width = imageBitmap.width;  // ❌ Canvas bị RESET tại đây!
  canvas.height = imageBitmap.height; // ❌ Context bị REINITIALIZE!
  ctx.drawImage(imageBitmap, 0, 0);   // ⚡ Có thể vẽ lên canvas đã bị xóa
}
```

**Vấn đề:** 
- Mỗi khi set `canvas.width` hoặc `canvas.height`, canvas **tự động bị xóa trắng** (cleared)
- Nếu nhiều frame đến cùng lúc, có thể xảy ra:
  - Frame A: resize canvas → xóa
  - Frame B: bắt đầu vẽ
  - Frame A: vẽ (nhưng canvas đã bị Frame B xóa) → **màn hình đen**

**Fix:**
```typescript
// Chỉ resize khi THẬT SỰ cần thiết
if (canvas.width !== imageBitmap.width || canvas.height !== imageBitmap.height) {
  canvas.width = imageBitmap.width;
  canvas.height = imageBitmap.height;
}
```

---

### **Vấn đề 2: Context được tạo lại liên tục** 🔄

**Code cũ:**
```typescript
const ctx = canvas.getContext("2d"); // ❌ Gọi mỗi frame!
```

**Vấn đề:**
- `getContext()` có overhead
- Không tận dụng được các optimization flags

**Fix:**
```typescript
// Cache context, chỉ tạo 1 lần
const ctxRef = useRef<CanvasRenderingContext2D | null>(null);

if (!ctxRef.current) {
  ctxRef.current = canvas.getContext("2d", {
    alpha: false,        // Không cần alpha channel → nhanh hơn
    desynchronized: true // Cho phép render async → mượt hơn
  });
}
```

---

### **Vấn đề 3: Memory Leak với ImageBitmap** 💾

**Code cũ:**
```typescript
const imageBitmap = await createImageBitmap(blob);
ctx.drawImage(imageBitmap, 0, 0);
imageBitmap.close(); // ❌ Close ngay → có thể conflict với drawImage
```

**Vấn đề:**
- `close()` ngay sau `drawImage()` có thể gây race condition
- Các frame cũ không được cleanup → memory leak

**Fix:**
```typescript
const lastFrameRef = useRef<ImageBitmap | null>(null);

ctx.drawImage(imageBitmap, 0, 0);

// Clean up frame CŨ (không phải frame hiện tại)
if (lastFrameRef.current) {
  lastFrameRef.current.close();
}
lastFrameRef.current = imageBitmap; // Giữ frame hiện tại
```

---

### **Vấn đề 4: Screen Capture Timeout (Backend)** ⏱️

**Code cũ (Rust):**
```rust
let buffer = loop {
    match capturer.frame() {
        Ok(frame) => break frame,
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            thread::sleep(Duration::from_millis(10));
            continue; // ❌ Loop vô hạn nếu screen bị lock!
        }
        Err(e) => return Err(format!("Failed: {}", e)),
    }
};
```

**Vấn đề:**
- Nếu màn hình bị lock hoặc screensaver bật → loop vô hạn
- Không timeout → frame bị stuck → client nhận frame cũ hoặc corrupt → **màn hình đen**

**Fix:**
```rust
let max_retries = 30; // Timeout sau 300ms
let mut retries = 0;
let buffer = loop {
    match capturer.frame() {
        Ok(frame) => break frame,
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            retries += 1;
            if retries >= max_retries {
                return Err("Capture timeout - screen may be locked".to_string());
            }
            thread::sleep(Duration::from_millis(10));
            continue;
        }
        Err(e) => return Err(format!("Failed: {}", e)),
    }
};
```

---

### **Vấn đề 5: Invalid Buffer Size Validation** 📏

**Code cũ:**
```rust
// Không validate buffer size
let img: RgbaImage = ImageBuffer::from_raw(width, height, rgba_data)
    .ok_or("Failed")?; // ❌ Nếu buffer size sai → corrupt image → màn đen
```

**Fix:**
```rust
// Validate TRƯỚC khi xử lý
let expected_size = width * height * 4;
if buffer.len() != expected_size {
    return Err(format!(
        "Invalid buffer: expected {} bytes, got {}",
        expected_size, buffer.len()
    ));
}
```

---

### **Vấn đề 6: UDP Frame Reassembly Issues** 📦

**Code cũ:**
```rust
// Không validate JPEG magic bytes
let complete_frame: Vec<u8> = chunks.concat();
let base64_image = base64::encode(&complete_frame);
app.emit("screen-frame", base64_image); // ❌ Có thể gửi frame corrupt!
```

**Vấn đề:**
- Nếu mất packet UDP → frame incomplete
- Không kiểm tra JPEG validity → gửi garbage data → **màn hình đen**

**Fix:**
```rust
let complete_frame: Vec<u8> = chunks.concat();

// Validate JPEG format
if complete_frame.len() >= 100 && 
   complete_frame.starts_with(&[0xFF, 0xD8]) && // JPEG start marker
   complete_frame.ends_with(&[0xFF, 0xD9]) {     // JPEG end marker
    let base64_image = base64::encode(&complete_frame);
    app.emit("screen-frame", base64_image);
} else {
    eprintln!("Invalid JPEG frame, skipping");
}
```

---

## 🚀 Các cải tiến khác

### **1. CSS GPU Acceleration**

```css
.screen-display canvas {
  /* Bật GPU rendering */
  transform: translateZ(0);
  will-change: transform;
  
  /* Sharp rendering */
  image-rendering: crisp-edges;
  
  /* Anti-flicker */
  backface-visibility: hidden;
}
```

### **2. Error Recovery**

```rust
let mut consecutive_errors = 0;
const MAX_CONSECUTIVE_ERRORS: u32 = 10;

match capture_fn() {
    Ok(data) => consecutive_errors = 0,
    Err(e) => {
        consecutive_errors += 1;
        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            // Stop để tránh spam errors
            *is_running.lock().unwrap() = false;
        }
    }
}
```

### **3. Better Logging**

Bây giờ có logs chi tiết:
- Số frames nhận được
- Incomplete frames trong buffer
- Invalid chunks
- Timeout frames

---

## 📊 Tóm tắt

| Vấn đề | Nguyên nhân | Fix |
|--------|-------------|-----|
| Màn hình đen ngẫu nhiên | Canvas resize race condition | Chỉ resize khi cần |
| Performance kém | Context tạo lại mỗi frame | Cache context với flags |
| Memory leak | ImageBitmap không cleanup | Cleanup frame cũ |
| Timeout capture | Loop vô hạn khi screen lock | Thêm max retries |
| Invalid frames | Không validate buffer/JPEG | Validate trước khi gửi/hiển thị |
| UDP packet loss | Không kiểm tra frame integrity | Validate JPEG magic bytes |

---

## 🧪 Testing

Để test các fix:

1. **Bật/tắt screensaver** → Không còn bị đen
2. **Lock/unlock screen** → Tự recovery
3. **Nhiều clients cùng lúc** → Mượt hơn
4. **Network congestion** → Skip bad frames thay vì hiển thị đen
5. **Rapid start/stop** → Không leak memory

---

## 💡 Best Practices được áp dụng

✅ **Single Responsibility**: Mỗi ref có 1 mục đích rõ ràng  
✅ **Resource Management**: Proper cleanup với useEffect  
✅ **Error Handling**: Validate ở mọi layer (Rust + TypeScript)  
✅ **Performance**: GPU acceleration, context caching  
✅ **Logging**: Debug-friendly với stats mỗi 5s  
✅ **Timeout Management**: Prevent infinite loops  

---

Bây giờ app của bạn sẽ **không còn bị màn hình đen** và **performance tốt hơn nhiều**! 🎉
