// Mouse Cursor Capture for Windows
// Draws cursor on captured frames

#[cfg(windows)]
use windows::Win32::{
    UI::WindowsAndMessaging::*,
    Graphics::Gdi::*,
    Foundation::*,
};

#[derive(Debug, Clone)]
pub struct CursorInfo {
    pub x: i32,
    pub y: i32,
    pub visible: bool,
    pub icon_data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

#[cfg(windows)]
pub struct CursorCapturer {
    last_cursor: Option<HCURSOR>,
}

#[cfg(windows)]
impl CursorCapturer {
    pub fn new() -> Self {
        Self {
            last_cursor: None,
        }
    }

    /// Get current cursor information
    pub fn get_cursor_info(&mut self) -> Option<CursorInfo> {
        unsafe {
            // Get cursor position and visibility
            let mut cursor_info = CURSORINFO {
                cbSize: std::mem::size_of::<CURSORINFO>() as u32,
                ..Default::default()
            };

            if GetCursorInfo(&mut cursor_info).is_err() {
                return None;
            }

            if cursor_info.flags != CURSOR_SHOWING {
                return Some(CursorInfo {
                    x: cursor_info.ptScreenPos.x,
                    y: cursor_info.ptScreenPos.y,
                    visible: false,
                    icon_data: None,
                    width: 0,
                    height: 0,
                });
            }

            // Get cursor icon info
            let mut icon_info = ICONINFO::default();
            if GetIconInfo(cursor_info.hCursor, &mut icon_info).is_err() {
                return Some(CursorInfo {
                    x: cursor_info.ptScreenPos.x,
                    y: cursor_info.ptScreenPos.y,
                    visible: true,
                    icon_data: None,
                    width: 0,
                    height: 0,
                });
            }

            // Get cursor bitmap dimensions
            let mut bitmap = BITMAP::default();
            if GetObjectW(
                icon_info.hbmColor,
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bitmap as *mut _ as *mut _),
            ) == 0 {
                // Monochrome cursor
                if GetObjectW(
                    icon_info.hbmMask,
                    std::mem::size_of::<BITMAP>() as i32,
                    Some(&mut bitmap as *mut _ as *mut _),
                ) == 0 {
                    return None;
                }
            }

            let width = bitmap.bmWidth as u32;
            let height = bitmap.bmHeight as u32;

            // For simplicity, we'll return cursor info without icon data
            // Full implementation would convert HBITMAP to RGBA
            // See RustDesk's implementation for full details

            // Cleanup
            if !icon_info.hbmColor.is_invalid() {
                DeleteObject(icon_info.hbmColor).ok();
            }
            if !icon_info.hbmMask.is_invalid() {
                DeleteObject(icon_info.hbmMask).ok();
            }

            Some(CursorInfo {
                x: cursor_info.ptScreenPos.x - icon_info.xHotspot as i32,
                y: cursor_info.ptScreenPos.y - icon_info.yHotspot as i32,
                visible: true,
                icon_data: None, // TODO: Convert HBITMAP to RGBA
                width,
                height,
            })
        }
    }

    /// Draw cursor onto RGBA frame buffer
    pub fn draw_cursor_on_frame(
        &mut self,
        frame: &mut [u8],
        frame_width: usize,
        frame_height: usize,
        display_x: i32,
        display_y: i32,
    ) {
        if let Some(cursor) = self.get_cursor_info() {
            if !cursor.visible {
                return;
            }

            // Simple cross-hair cursor for now
            // Full implementation would draw actual cursor icon
            let cursor_x = (cursor.x - display_x) as usize;
            let cursor_y = (cursor.y - display_y) as usize;

            self.draw_crosshair(frame, frame_width, frame_height, cursor_x, cursor_y);
        }
    }

    /// Draw a simple crosshair (placeholder for actual cursor)
    fn draw_crosshair(
        &self,
        frame: &mut [u8],
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) {
        let size = 10; // crosshair size
        let color = [255u8, 0, 0, 255]; // Red with full opacity

        // Draw horizontal line
        for dx in 0..size {
            let px = x.saturating_add(dx).saturating_sub(size / 2);
            if px < width && y < height {
                let idx = (y * width + px) * 4;
                if idx + 3 < frame.len() {
                    frame[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }

        // Draw vertical line
        for dy in 0..size {
            let py = y.saturating_add(dy).saturating_sub(size / 2);
            if x < width && py < height {
                let idx = (py * width + x) * 4;
                if idx + 3 < frame.len() {
                    frame[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }
    }
}

#[cfg(not(windows))]
pub struct CursorCapturer;

#[cfg(not(windows))]
impl CursorCapturer {
    pub fn new() -> Self {
        Self
    }

    pub fn get_cursor_info(&mut self) -> Option<CursorInfo> {
        None
    }

    pub fn draw_cursor_on_frame(
        &mut self,
        _frame: &mut [u8],
        _frame_width: usize,
        _frame_height: usize,
        _display_x: i32,
        _display_y: i32,
    ) {
        // Not supported on non-Windows platforms
    }
}

// Utility function to draw cursor on frame
pub fn draw_cursor(
    frame: &mut [u8],
    frame_width: usize,
    frame_height: usize,
    display_x: i32,
    display_y: i32,
) {
    let mut capturer = CursorCapturer::new();
    capturer.draw_cursor_on_frame(frame, frame_width, frame_height, display_x, display_y);
}
