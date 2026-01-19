use scrap::{Capturer, Display};
use image::{ImageBuffer, RgbaImage, DynamicImage};
use std::io::Cursor;
use std::thread;
use std::time::Duration;

const JPEG_QUALITY: u8 = 50; // Lower quality for smaller packets
const MAX_WIDTH: u32 = 1280; // Scale down large screens

pub fn capture_screen() -> Result<Vec<u8>, String> {
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
    let expected_size = width * height * 4; // BGRA = 4 bytes per pixel
    if buffer.len() != expected_size {
        return Err(format!(
            "Invalid buffer size: expected {} bytes for {}x{} display, got {} bytes",
            expected_size, width, height, buffer.len()
        ));
    }
    
    // Convert BGRA to RGBA
    let mut rgba_data = Vec::with_capacity(buffer.len());
    for chunk in buffer.chunks_exact(4) {
        rgba_data.push(chunk[2]); // R
        rgba_data.push(chunk[1]); // G
        rgba_data.push(chunk[0]); // B
        rgba_data.push(chunk[3]); // A
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
