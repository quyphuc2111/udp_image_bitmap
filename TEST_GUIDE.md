# 🧪 Test Guide - Verify Fixes

## 🎯 Mục tiêu testing

Verify rằng các fixes đã hoạt động đúng và không còn vấn đề màn hình đen.

---

## 📝 Test Cases

### ✅ Test 1: Normal Operation
**Mục đích**: Verify streaming hoạt động mượt mà

**Steps**:
1. Mở terminal, chạy: `npm run tauri dev`
2. Chọn **Server Mode**
3. Click **"Bắt đầu chia sẻ"**
4. Mở thêm 1 cửa sổ khác, chọn **Client Mode**
5. Click **"Kết nối"**
6. Quan sát màn hình hiển thị

**Expected**:
- ✅ Màn hình hiển thị mượt mà (~10 FPS)
- ✅ Không bị đen
- ✅ Không lag đột ngột

**Logs to check**:
```
📊 Stats: X frames received, Y incomplete frames in buffer
```

---

### ✅ Test 2: Screen Lock/Unlock
**Mục đích**: Verify recovery sau khi lock màn hình

**Steps**:
1. Start Server + Client như Test 1
2. Lock màn hình máy Server (Cmd+Ctrl+Q trên macOS)
3. Đợi 5 giây
4. Unlock màn hình
5. Quan sát màn hình Client

**Expected**:
- ✅ Client hiển thị "Capture timeout" trong console (bình thường)
- ✅ Sau unlock, stream tự động recovery trong <1s
- ✅ Không bị freeze hoặc crash

**Logs to check**:
```
Capture error (1/10): Capture timeout after 30 retries - screen may be locked
```

---

### ✅ Test 3: Rapid Start/Stop
**Mục đích**: Verify không bị memory leak

**Steps**:
1. Start Client
2. Start Server → Stop Server → Start Server (repeat 10 lần nhanh)
3. Quan sát memory usage

**Expected**:
- ✅ Memory không tăng liên tục
- ✅ Không crash
- ✅ Canvas được clear đúng cách mỗi lần stop

**How to check memory**:
- macOS: Activity Monitor → Memory tab
- Memory should stabilize after ~5 cycles

---

### ✅ Test 4: Multiple Clients
**Mục đích**: Verify multicast hoạt động đúng

**Steps**:
1. Start 1 Server
2. Start 3 Clients (3 cửa sổ khác nhau)
3. Quan sát tất cả clients

**Expected**:
- ✅ Tất cả clients đều nhận được stream
- ✅ Frame rate ổn định (~10 FPS mỗi client)
- ✅ Không bị đen trên bất kỳ client nào

---

### ✅ Test 5: Network Congestion Simulation
**Mục đích**: Verify xử lý packet loss

**Steps**:
1. Start Server + Client
2. Mở nhiều apps tốn network (YouTube, download file lớn)
3. Quan sát Client display

**Expected**:
- ✅ Có thể bị giật lag (bình thường)
- ✅ KHÔNG bị màn hình đen hoàn toàn
- ✅ Logs show "Discarding incomplete frame" (normal)

**Logs to check**:
```
Discarding incomplete frame 123 (timeout)
Cleaned up 3 incomplete frames
```

---

### ✅ Test 6: Long Running (Memory Leak Check)
**Mục đích**: Verify không leak memory sau thời gian dài

**Steps**:
1. Start Server + Client
2. Để chạy liên tục 30 phút
3. Check memory usage mỗi 10 phút

**Expected**:
- ✅ Memory tăng ban đầu rồi ổn định
- ✅ Không crash
- ✅ Frame rate vẫn ổn định

**Memory benchmarks**:
- Initial: ~50-80MB
- After 10 min: ~70-100MB
- After 30 min: ~80-120MB (should stabilize)

---

### ✅ Test 7: Invalid Frame Handling
**Mục đích**: Verify xử lý corrupt frames

**Steps**:
1. Start Server + Client
2. Quan sát console logs
3. Tìm các warnings về invalid frames

**Expected Logs**:
```
Warning: Invalid JPEG frame (size: 45, starts: [00 00])
Received packet too small: 8 bytes
```

**Expected behavior**:
- ✅ Invalid frames được skip
- ✅ Stream continues với frame tiếp theo
- ✅ Không crash

---

## 🔍 Debug Commands

### Check if multicast is working:
```bash
# Terminal 1 - Start server
npm run tauri dev

# Terminal 2 - Monitor multicast traffic (macOS)
sudo tcpdump -i en0 host 239.0.0.1 -c 100
```

### Monitor memory usage:
```bash
# macOS
top -pid $(pgrep -f "screensharing")

# Linux
htop -p $(pgrep -f "screensharing")
```

### Check logs:
```bash
# Development mode shows logs in terminal
npm run tauri dev

# Look for:
# ✅ "📊 Stats: X frames received..."
# ✅ "Discarding incomplete frame..."
# ❌ "Too many consecutive capture errors"
```

---

## 📊 Performance Checklist

Mark ✅ if passing:

- [ ] **Render time**: < 10ms per frame (check DevTools)
- [ ] **Frame rate**: ~10 FPS stable
- [ ] **Memory**: < 150MB after 30 min
- [ ] **Recovery**: < 1s after unlock screen
- [ ] **Black screens**: 0-1 per 10 min (vs 5-10 before)
- [ ] **CPU usage**: < 20% on modern hardware

---

## 🐛 Common Issues & Solutions

### Issue: "Failed to bind socket"
**Solution**: Port 9999 đã được sử dụng
```bash
# Kill existing process
lsof -ti:9999 | xargs kill -9
```

### Issue: "Failed to get primary display"
**Solution**: Cần screen recording permission
1. System Preferences → Security & Privacy → Screen Recording
2. Enable cho app của bạn

### Issue: Canvas vẫn bị đen
**Check**:
1. Console có logs "Invalid JPEG frame" không?
2. Network có packet loss không? (tcpdump)
3. Server có log "Capture timeout" không?

---

## ✅ Success Criteria

**Tất cả tests pass nếu**:
1. ✅ Stream mượt mà ít nhất 5 phút liên tục
2. ✅ Không có black screen hoàn toàn (chỉ có thể giật lag)
3. ✅ Recovery sau screen lock < 2s
4. ✅ Memory stable sau 30 phút
5. ✅ Logs không có "Too many consecutive errors"

---

**Good luck testing! 🚀**

Nếu gặp vấn đề, check file `FIXES_EXPLANATION.md` để hiểu rõ hơn về các fixes.
