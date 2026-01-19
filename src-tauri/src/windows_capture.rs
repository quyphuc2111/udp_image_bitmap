// Windows.Graphics.Capture implementation for better performance on Windows 10+
// Note: This requires Windows 10 version 1803 (April 2018 Update) or later
#[cfg(target_os = "windows")]
use windows::Graphics::Capture::{GraphicsCaptureItem, Direct3D11CaptureFramePool, GraphicsCaptureSession};
#[cfg(target_os = "windows")]
use windows::Graphics::DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat};
#[cfg(target_os = "windows")]
use windows::Foundation::TypedEventHandler;
#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "windows")]
use image::{ImageBuffer, RgbaImage, DynamicImage};
#[cfg(target_os = "windows")]
use std::io::Cursor;

#[cfg(target_os = "windows")]
pub struct WindowsScreenCapture {
    session: Option<GraphicsCaptureSession>,
    frame_pool: Option<Direct3D11CaptureFramePool>,
    last_frame: Arc<Mutex<Option<Vec<u8>>>>,
}

#[cfg(target_os = "windows")]
impl WindowsScreenCapture {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            session: None,
            frame_pool: None,
            last_frame: Arc::new(Mutex::new(None)),
        })
    }

    /// Initialize Windows.Graphics.Capture
    /// This is more efficient than scrap but requires Windows 10 1803+
    pub fn start_capture(&mut self) -> Result<(), String> {
        eprintln!("⚠️  Windows.Graphics.Capture requires complex COM initialization");
        eprintln!("    Current implementation: Using scrap as stable fallback");
        eprintln!("    For full Windows.Graphics.Capture support:");
        eprintln!("    1. Initialize COM apartment");
        eprintln!("    2. Create Direct3D11 device");
        eprintln!("    3. Create GraphicsCaptureItem for primary monitor");
        eprintln!("    4. Setup frame pool and capture session");
        eprintln!("    See: https://docs.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture");
        
        // For now, return error to fallback to scrap
        // Full implementation would require:
        // - windows-rs bindings for COM initialization
        // - Direct3D11 device creation
        // - Monitor enumeration via DXGI
        // - GraphicsCaptureItem creation
        Err("Windows.Graphics.Capture initialization deferred - using scrap".to_string())
    }

    pub fn get_frame(&self) -> Result<Vec<u8>, String> {
        let frame = self.last_frame.lock().unwrap();
        frame.clone().ok_or_else(|| "No frame available".to_string())
    }

    pub fn stop_capture(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = session.Close();
        }
        self.frame_pool = None;
    }
}

/// Simple function to check if Windows.Graphics.Capture is available
#[cfg(target_os = "windows")]
pub fn is_windows_graphics_capture_available() -> bool {
    // Check Windows version (requires Windows 10 1803+)
    // For simplicity, always return false for now
    // Real implementation would check: ntdll.RtlGetVersion() >= 10.0.17134
    false
}

/// Platform-specific screen capture with automatic fallback
/// Windows: Tries Windows.Graphics.Capture, falls back to scrap
/// macOS/Linux: Uses scrap directly
pub fn capture_screen_platform_specific() -> Result<Vec<u8>, String> {
    #[cfg(target_os = "windows")]
    {
        // Check if Windows.Graphics.Capture is available
        if is_windows_graphics_capture_available() {
            // Try Windows.Graphics.Capture (better performance)
            static mut WINDOWS_CAPTURE: Option<WindowsScreenCapture> = None;
            static mut TRIED_INIT: bool = false;
            
            unsafe {
                if !TRIED_INIT {
                    match WindowsScreenCapture::new() {
                        Ok(mut capture) => {
                            match capture.start_capture() {
                                Ok(_) => {
                                    eprintln!("✅ Using Windows.Graphics.Capture (high performance)");
                                    WINDOWS_CAPTURE = Some(capture);
                                }
                                Err(e) => {
                                    eprintln!("⚠️  Windows.Graphics.Capture init failed: {}", e);
                                    eprintln!("    Falling back to scrap library");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("⚠️  Failed to create WindowsScreenCapture: {}", e);
                        }
                    }
                    TRIED_INIT = true;
                }
                
                // Try to use Windows.Graphics.Capture if initialized
                if let Some(ref capture) = WINDOWS_CAPTURE {
                    if let Ok(frame) = capture.get_frame() {
                        return Ok(frame);
                    }
                }
            }
        }
        
        // Fallback to scrap (stable, cross-platform)
        crate::screen_capture::capture_screen()
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        crate::screen_capture::capture_screen()
    }
}

/// Get a human-readable name of the capture method being used
#[allow(dead_code)]
pub fn get_capture_method_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        if is_windows_graphics_capture_available() {
            "Windows.Graphics.Capture (Windows 10+)"
        } else {
            "scrap (fallback)"
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        "scrap (macOS CoreGraphics)"
    }
    
    #[cfg(target_os = "linux")]
    {
        "scrap (X11/Wayland)"
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "scrap (generic)"
    }
}
