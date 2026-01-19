use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

const MULTICAST_ADDR: &str = "239.0.0.1:9999";
const CHUNK_SIZE: usize = 8192; // Smaller chunks for UDP safety (8KB)
const JPEG_QUALITY: u8 = 60; // Lower quality for smaller size
const REDUNDANT_PACKETS: bool = true; // Send critical packets twice for reliability

pub struct UdpServer {
    socket: Arc<UdpSocket>,
    is_running: Arc<Mutex<bool>>,
}

impl UdpServer {
    pub fn new() -> Result<Self, String> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| format!("Failed to bind socket: {}", e))?;
        
        socket.set_multicast_ttl_v4(32)
            .map_err(|e| format!("Failed to set TTL: {}", e))?;
        
        Ok(Self {
            socket: Arc::new(socket),
            is_running: Arc::new(Mutex::new(false)),
        })
    }
    
    pub async fn start_streaming<F>(&self, capture_fn: F) -> Result<(), String>
    where
        F: Fn() -> Result<Vec<u8>, String> + Send + 'static,
    {
        *self.is_running.lock().unwrap() = true;
        let socket = self.socket.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            let mut frame_id = 0u32;
            let mut consecutive_errors = 0u32;
            const MAX_CONSECUTIVE_ERRORS: u32 = 10;
            
            while *is_running.lock().unwrap() {
                match capture_fn() {
                    Ok(data) => {
                        // Reset error counter on success
                        consecutive_errors = 0;
                        
                        // Skip empty frames (black screens)
                        if data.is_empty() || data.len() < 100 {
                            eprintln!("Warning: Captured frame is too small ({} bytes), skipping", data.len());
                            sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                        
                        // Compress more if still too large
                        let compressed = if data.len() > 500_000 {
                            match Self::recompress_jpeg(&data, JPEG_QUALITY) {
                                Ok(d) => d,
                                Err(e) => {
                                    eprintln!("Recompress error: {}", e);
                                    sleep(Duration::from_millis(100)).await;
                                    continue;
                                }
                            }
                        } else {
                            data
                        };
                        
                        if let Err(e) = Self::send_chunked(&socket, &compressed, frame_id).await {
                            eprintln!("Send error: {}", e);
                        } else {
                            // Only increment frame ID on successful send
                            frame_id = frame_id.wrapping_add(1);
                        }
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        eprintln!("Capture error ({}/{}): {}", consecutive_errors, MAX_CONSECUTIVE_ERRORS, e);
                        
                        // Stop streaming if too many consecutive errors
                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            eprintln!("Too many consecutive capture errors. Stopping stream.");
                            *is_running.lock().unwrap() = false;
                            break;
                        }
                    }
                }
                sleep(Duration::from_millis(100)).await; // ~10 FPS
            }
        });
        
        Ok(())
    }
    
    fn recompress_jpeg(data: &[u8], quality: u8) -> Result<Vec<u8>, String> {
        use image::ImageReader;
        use std::io::Cursor;
        
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?;
        
        let mut buffer = Cursor::new(Vec::new());
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
        encoder.encode(
            img.as_bytes(),
            img.width(),
            img.height(),
            img.color().into()
        ).map_err(|e| e.to_string())?;
        
        Ok(buffer.into_inner())
    }
    
    async fn send_chunked(socket: &UdpSocket, data: &[u8], frame_id: u32) -> Result<(), String> {
        let total_chunks = (data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let chunks: Vec<&[u8]> = data.chunks(CHUNK_SIZE).collect();
        
        // First pass: Send all chunks
        for (i, chunk) in chunks.iter().enumerate() {
            let mut packet = Vec::with_capacity(12 + chunk.len());
            packet.extend_from_slice(&frame_id.to_be_bytes());
            packet.extend_from_slice(&(i as u32).to_be_bytes());
            packet.extend_from_slice(&(total_chunks as u32).to_be_bytes());
            packet.extend_from_slice(chunk);
            
            socket.send_to(&packet, MULTICAST_ADDR)
                .map_err(|e| format!("Send failed: {}", e))?;
            
            // Small delay between chunks to avoid overwhelming network
            if i % 10 == 0 {
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        }
        
        // Second pass: Resend first and last chunks for reliability (critical for JPEG)
        if REDUNDANT_PACKETS && total_chunks > 2 {
            tokio::time::sleep(Duration::from_micros(500)).await;
            
            // Resend first chunk (JPEG header)
            if let Some(first_chunk) = chunks.first() {
                let mut packet = Vec::with_capacity(12 + first_chunk.len());
                packet.extend_from_slice(&frame_id.to_be_bytes());
                packet.extend_from_slice(&0u32.to_be_bytes());
                packet.extend_from_slice(&(total_chunks as u32).to_be_bytes());
                packet.extend_from_slice(first_chunk);
                let _ = socket.send_to(&packet, MULTICAST_ADDR);
            }
            
            // Resend last chunk (JPEG end marker)
            if let Some(last_chunk) = chunks.last() {
                let last_idx = chunks.len() - 1;
                let mut packet = Vec::with_capacity(12 + last_chunk.len());
                packet.extend_from_slice(&frame_id.to_be_bytes());
                packet.extend_from_slice(&(last_idx as u32).to_be_bytes());
                packet.extend_from_slice(&(total_chunks as u32).to_be_bytes());
                packet.extend_from_slice(last_chunk);
                let _ = socket.send_to(&packet, MULTICAST_ADDR);
            }
        }
        
        Ok(())
    }
    
    pub fn stop(&self) {
        *self.is_running.lock().unwrap() = false;
    }
}
