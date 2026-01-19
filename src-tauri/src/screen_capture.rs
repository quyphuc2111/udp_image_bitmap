use scrap::{Capturer, Display};
use image::{ImageBuffer, RgbaImage, DynamicImage};
use std::io::Cursor;
use std::thread;
use std::time::Duration;

const JPEG_QUALITY: u8 = 50; // Lower quality for smaller packets
const MAX_WIDTH: u32 = 1280; // Scale down large screens

#[cfg(all(target_os = "windows", feature = "dxgi"))]
use std::sync::Mutex;
#[cfg(all(target_os = "windows", feature = "dxgi"))]
use crate::dxgi_capture::DxgiCapturer;

#[cfg(all(target_os = "windows", feature = "dxgi"))]
static DXGI_CAPTURER: Mutex<Option<DxgiCapturer>> = Mutex::new(None);
#[cfg(all(target_os = "windows", feature = "dxgi"))]
static TRIED_DXGI: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn capture_screen() -> Result<Vec<u8>, String> {
    #[cfg(all(target_os = "windows", feature = "dxgi"))]
    {
        // Try DXGI capture first (10x faster than scrap on Windows)
        if !TRIED_DXGI.load(std::sync::atomic::Ordering::Relaxed) {
            if crate::dxgi_capture::is_dxgi_available() {
                match crate::dxgi_capture::create_dxgi_capturer(0) {
                    Ok(capturer) => {
                        eprintln!("✅ Using DXGI Desktop Duplication (high performance)");
                        *DXGI_CAPTURER.lock().unwrap() = Some(capturer);
                    }
                    Err(e) => {
                        eprintln!("⚠️  DXGI init failed: {}", e);
                        eprintln!("   Falling back to scrap library");
                    }
                }
            } else {
                eprintln!("ℹ️  DXGI not available, using scrap library");
            }
            TRIED_DXGI.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        // Try to use DXGI if initialized
        let mut dxgi_guard = DXGI_CAPTURER.lock().unwrap();
        if let Some(ref mut capturer) = *dxgi_guard {
            match capturer.capture_frame() {
                Ok(rgba_data) => {
                    // Successfully captured with DXGI
                    return encode_rgba_to_jpeg(
                        &rgba_data,
                        capturer.width(),
                        capturer.height(),
                    );
                }
                Err(e) if e == "WouldBlock" => {
                    // No new frame available, this is normal
                    return Err("WouldBlock".to_string());
                }
                Err(e) => {
                    eprintln!("❌ DXGI capture error: {}, switching to scrap", e);
                    *dxgi_guard = None; // Disable DXGI, fallback to scrap
                }
            }
        }
        drop(dxgi_guard);
    }

    // Fallback to scrap (always available on all platforms)
    capture_screen_scrap()
}

// Original scrap-based capture (fallback)
fn capture_screen_scrap() -> Result<Vec<u8>, String> {
    // Get primary display
    let display = Display::primary()
        .map_err(|e| format!("Failed to get primary display: {}", e))?;
    
    let width = display.width();
    let height = display.height();
    
    // Create capturer
    let mut capturer = Capturer::new(display)
        .map_err(|e| format!("Failed to create capturer: {}", e))?;
    
    // Try to capture frame with timeout to prevent infinite loops
    let max_retries = 30; // Max 300ms wait
    let buffer = {
        let mut retries = 0;
        loop {
            match capturer.frame() {
                Ok(frame) => break frame,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    retries += 1;
                    if retries >= max_retries {
                        return Err(format!("Capture timeout after {} retries - screen may be locked or unavailable", max_retries));
                    }
                    // Frame not ready yet, wait a bit
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(format!("Failed to capture frame: {}", e)),
            }
        }
    };
    
    // Validate buffer size before processing
    // Note: Buffer may have padding/stride, so it can be larger than expected
    let min_expected_size = width * height * 4; // BGRA = 4 bytes per pixel
    if buffer.len() < min_expected_size {
        return Err(format!(
            "Invalid buffer size: expected at least {} bytes for {}x{} display, got {} bytes",
            min_expected_size, width, height, buffer.len()
        ));
    }
    
    // Calculate actual stride (bytes per row)
    let stride = buffer.len() / height;
    if stride < width * 4 {
        return Err(format!(
            "Invalid stride: {} bytes per row, expected at least {} for width {}",
            stride, width * 4, width
        ));
    }
    
    // Convert BGRA to RGBA, handling stride properly
    let mut rgba_data = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let row_start = y * stride;
        for x in 0..width {
            let pixel_offset = row_start + x * 4;
            if pixel_offset + 3 < buffer.len() {
                rgba_data.push(buffer[pixel_offset + 2]); // R
                rgba_data.push(buffer[pixel_offset + 1]); // G
                rgba_data.push(buffer[pixel_offset]);     // B
                rgba_data.push(buffer[pixel_offset + 3]); // A
            }
        }
    }
    
    // Create image
    let img: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, rgba_data)
        .ok_or("Failed to create image buffer - invalid dimensions or data")?;
    
    let mut dynamic_img = DynamicImage::ImageRgba8(img);
    
    // Scale down if too large
    if width as u32 > MAX_WIDTH {
        let scale = MAX_WIDTH as f32 / width as f32;
        let new_height = (height as f32 * scale) as u32;
        dynamic_img = dynamic_img.resize(MAX_WIDTH, new_height, image::imageops::FilterType::Lanczos3);
    }
    
    // Convert RGBA to RGB (JPEG doesn't support alpha channel)
    let rgb_img = dynamic_img.to_rgb8();
    
    // Encode to JPEG with compression
    let mut buffer = Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, JPEG_QUALITY);
    encoder.encode(
        rgb_img.as_raw(),
        rgb_img.width(),
        rgb_img.height(),
        image::ExtendedColorType::Rgb8
    ).map_err(|e| format!("Failed to encode JPEG: {}", e))?;
    
    Ok(buffer.into_inner())
}

// Helper function to encode RGBA to JPEG
fn encode_rgba_to_jpeg(rgba: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    // Convert RGBA to RGB
    let mut rgb = Vec::with_capacity(width * height * 3);
    for chunk in rgba.chunks_exact(4) {
        rgb.push(chunk[0]); // R
        rgb.push(chunk[1]); // G
        rgb.push(chunk[2]); // B
    }

    let img: image::RgbImage = ImageBuffer::from_raw(width as u32, height as u32, rgb)
        .ok_or("Failed to create RGB image buffer")?;

    let mut dynamic_img = DynamicImage::ImageRgb8(img);

    // Scale down if too large
    if width as u32 > MAX_WIDTH {
        let scale = MAX_WIDTH as f32 / width as f32;
        let new_height = (height as f32 * scale) as u32;
        dynamic_img = dynamic_img.resize(MAX_WIDTH, new_height, image::imageops::FilterType::Lanczos3);
    }

    // Encode to JPEG
    let mut buffer = Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, JPEG_QUALITY);
    
    let rgb_img = dynamic_img.to_rgb8();
    encoder.encode(
        rgb_img.as_raw(),
        rgb_img.width(),
        rgb_img.height(),
        image::ExtendedColorType::Rgb8,
    ).map_err(|e| format!("Failed to encode JPEG: {}", e))?;

    Ok(buffer.into_inner())
}

// Alternative: Capture with quality control
pub fn capture_screen_with_quality(quality: u8) -> Result<Vec<u8>, String> {
    let display = Display::primary()
        .map_err(|e| format!("Failed to get primary display: {}", e))?;
    
    let width = display.width();
    let height = display.height();
    
    let mut capturer = Capturer::new(display)
        .map_err(|e| format!("Failed to create capturer: {}", e))?;
    
    let buffer = loop {
        match capturer.frame() {
            Ok(frame) => break frame,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(format!("Failed to capture frame: {}", e)),
        }
    };
    
    // Convert BGRA to RGBA
    let mut rgba_data = Vec::with_capacity(buffer.len());
    for chunk in buffer.chunks_exact(4) {
        rgba_data.push(chunk[2]); // R
        rgba_data.push(chunk[1]); // G
        rgba_data.push(chunk[0]); // B
        rgba_data.push(chunk[3]); // A
    }
    
    let img: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, rgba_data)
        .ok_or("Failed to create image buffer")?;
    
    // Convert RGBA to RGB (JPEG doesn't support alpha channel)
    let rgb_img = DynamicImage::ImageRgba8(img).to_rgb8();
    
    // Encode with custom quality
    let mut buffer = Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
    encoder.encode(
        rgb_img.as_raw(),
        width as u32,
        height as u32,
        image::ExtendedColorType::Rgb8
    ).map_err(|e| format!("Failed to encode JPEG: {}", e))?;
    
    Ok(buffer.into_inner())
}

// Get available displays
pub fn get_displays() -> Result<Vec<(usize, usize, usize)>, String> {
    let displays = Display::all()
        .map_err(|e| format!("Failed to get displays: {}", e))?;
    
    Ok(displays
        .iter()
        .enumerate()
        .map(|(idx, d)| (idx, d.width(), d.height()))
        .collect())
}
