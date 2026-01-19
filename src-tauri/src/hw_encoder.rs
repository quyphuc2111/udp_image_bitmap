// Hardware H264 Encoder wrapper
// Simplified version of RustDesk's hardware encoding

use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EncoderType {
    Software,      // JPEG
    HardwareH264,  // NVENC, QuickSync, AMF, VideoToolbox
    HardwareH265,  // HEVC
}

pub struct EncoderConfig {
    pub width: usize,
    pub height: usize,
    pub fps: u32,
    pub bitrate: u32,          // bps
    pub encoder_type: EncoderType,
    pub quality: u8,           // 1-100 for JPEG, or CRF for H264
}

pub trait VideoEncoder: Send {
    fn encode(&mut self, rgba: &[u8]) -> Result<Vec<u8>, String>;
    fn encoder_type(&self) -> EncoderType;
    fn set_bitrate(&mut self, bitrate: u32) -> Result<(), String>;
    fn set_fps(&mut self, fps: u32) -> Result<(), String>;
}

// JPEG Software Encoder (current implementation)
pub struct JpegEncoder {
    quality: u8,
    width: usize,
    height: usize,
}

impl JpegEncoder {
    pub fn new(config: &EncoderConfig) -> Result<Self, String> {
        Ok(Self {
            quality: config.quality,
            width: config.width,
            height: config.height,
        })
    }
}

impl VideoEncoder for JpegEncoder {
    fn encode(&mut self, rgba: &[u8]) -> Result<Vec<u8>, String> {
        // Convert RGBA to RGB
        let mut rgb = Vec::with_capacity(self.width * self.height * 3);
        for chunk in rgba.chunks_exact(4) {
            rgb.push(chunk[0]); // R
            rgb.push(chunk[1]); // G
            rgb.push(chunk[2]); // B
        }

        // Encode to JPEG
        use image::{ImageBuffer, RgbImage};
        use std::io::Cursor;

        let img: RgbImage = ImageBuffer::from_raw(
            self.width as u32,
            self.height as u32,
            rgb,
        ).ok_or("Failed to create image buffer")?;

        let mut buffer = Cursor::new(Vec::new());
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut buffer,
            self.quality,
        );

        encoder.encode(
            img.as_raw(),
            self.width as u32,
            self.height as u32,
            image::ExtendedColorType::Rgb8,
        ).map_err(|e| format!("JPEG encoding failed: {}", e))?;

        Ok(buffer.into_inner())
    }

    fn encoder_type(&self) -> EncoderType {
        EncoderType::Software
    }

    fn set_bitrate(&mut self, _bitrate: u32) -> Result<(), String> {
        // JPEG doesn't use bitrate, ignore
        Ok(())
    }

    fn set_fps(&mut self, _fps: u32) -> Result<(), String> {
        // JPEG is per-frame, FPS handled externally
        Ok(())
    }
}

// Hardware H264 Encoder (placeholder - requires platform-specific implementation)
#[cfg(feature = "hwcodec")]
pub struct H264HardwareEncoder {
    width: usize,
    height: usize,
    bitrate: u32,
    fps: u32,
    // Platform-specific encoder would go here
}

#[cfg(feature = "hwcodec")]
impl H264HardwareEncoder {
    pub fn new(config: &EncoderConfig) -> Result<Self, String> {
        // Check for hardware encoder availability
        if !Self::is_available() {
            return Err("Hardware H264 encoder not available".to_string());
        }

        eprintln!("🎬 Initializing hardware H264 encoder");
        eprintln!("   Resolution: {}x{}", config.width, config.height);
        eprintln!("   Bitrate: {} Mbps", config.bitrate / 1_000_000);
        eprintln!("   FPS: {}", config.fps);

        Ok(Self {
            width: config.width,
            height: config.height,
            bitrate: config.bitrate,
            fps: config.fps,
        })
    }

    pub fn is_available() -> bool {
        // Check for NVENC, QuickSync, AMF, etc.
        #[cfg(target_os = "windows")]
        {
            // Check for NVIDIA, Intel, AMD encoders
            // For now, return false (not implemented)
            false
        }
        #[cfg(target_os = "macos")]
        {
            // VideoToolbox is usually available
            false // Not implemented yet
        }
        #[cfg(target_os = "linux")]
        {
            // Check for VAAPI
            false
        }
    }
}

#[cfg(feature = "hwcodec")]
impl VideoEncoder for H264HardwareEncoder {
    fn encode(&mut self, _rgba: &[u8]) -> Result<Vec<u8>, String> {
        // TODO: Implement hardware encoding
        // This would use:
        // - NVENC on NVIDIA GPUs
        // - QuickSync on Intel
        // - AMF on AMD
        // - VideoToolbox on macOS
        // - VAAPI on Linux
        Err("Hardware H264 encoding not yet implemented".to_string())
    }

    fn encoder_type(&self) -> EncoderType {
        EncoderType::HardwareH264
    }

    fn set_bitrate(&mut self, bitrate: u32) -> Result<(), String> {
        self.bitrate = bitrate;
        // TODO: Update hardware encoder bitrate
        Ok(())
    }

    fn set_fps(&mut self, fps: u32) -> Result<(), String> {
        self.fps = fps;
        // TODO: Update hardware encoder FPS
        Ok(())
    }
}

// Encoder factory
pub fn create_encoder(config: EncoderConfig) -> Result<Box<dyn VideoEncoder>, String> {
    match config.encoder_type {
        EncoderType::Software => {
            eprintln!("📹 Using JPEG software encoder (quality: {})", config.quality);
            Ok(Box::new(JpegEncoder::new(&config)?))
        }
        #[cfg(feature = "hwcodec")]
        EncoderType::HardwareH264 => {
            match H264HardwareEncoder::new(&config) {
                Ok(encoder) => {
                    eprintln!("✅ Hardware H264 encoder initialized");
                    Ok(Box::new(encoder))
                }
                Err(e) => {
                    eprintln!("⚠️  Hardware encoder failed: {}, falling back to JPEG", e);
                    let jpeg_config = EncoderConfig {
                        encoder_type: EncoderType::Software,
                        ..config
                    };
                    Ok(Box::new(JpegEncoder::new(&jpeg_config)?))
                }
            }
        }
        #[cfg(not(feature = "hwcodec"))]
        EncoderType::HardwareH264 | EncoderType::HardwareH265 => {
            eprintln!("⚠️  Hardware encoding not compiled in, using JPEG");
            let jpeg_config = EncoderConfig {
                encoder_type: EncoderType::Software,
                ..config
            };
            Ok(Box::new(JpegEncoder::new(&jpeg_config)?))
        }
        EncoderType::HardwareH265 => {
            eprintln!("⚠️  H265 not implemented, using JPEG");
            let jpeg_config = EncoderConfig {
                encoder_type: EncoderType::Software,
                ..config
            };
            Ok(Box::new(JpegEncoder::new(&jpeg_config)?))
        }
    }
}

// Auto-detect best encoder
pub fn auto_detect_encoder(width: usize, height: usize, fps: u32) -> EncoderConfig {
    #[cfg(feature = "hwcodec")]
    {
        if H264HardwareEncoder::is_available() {
            eprintln!("🎯 Auto-detected: Hardware H264 encoder available");
            return EncoderConfig {
                width,
                height,
                fps,
                bitrate: calculate_bitrate(width, height, fps),
                encoder_type: EncoderType::HardwareH264,
                quality: 23, // CRF value
            };
        }
    }

    eprintln!("🎯 Auto-detected: Using JPEG software encoder");
    EncoderConfig {
        width,
        height,
        fps,
        bitrate: 0, // Not used for JPEG
        encoder_type: EncoderType::Software,
        quality: 70, // JPEG quality
    }
}

// Calculate appropriate bitrate for H264
fn calculate_bitrate(width: usize, height: usize, fps: u32) -> u32 {
    // Simple formula: pixels_per_second * bits_per_pixel
    // For H264, typically 0.1 - 0.2 bpp (bits per pixel)
    let pixels_per_second = (width * height * fps as usize) as u32;
    let bpp = 0.15; // 0.15 bits per pixel
    (pixels_per_second as f32 * bpp) as u32
}
