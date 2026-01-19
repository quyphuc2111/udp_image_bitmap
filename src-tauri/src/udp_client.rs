use std::collections::HashMap;
use std::net::{UdpSocket, Ipv4Addr};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, AppHandle};
use socket2::{Socket, Domain, Type, Protocol};

const FRAME_TIMEOUT_MS: u64 = 2000; // Discard incomplete frames after 2s

pub struct UdpClient {
    socket: Arc<UdpSocket>,
    is_running: Arc<Mutex<bool>>,
    frame_buffer: Arc<Mutex<HashMap<u32, (Vec<Vec<u8>>, std::time::Instant)>>>,
}

impl UdpClient {
    pub fn new() -> Result<Self, String> {
        // Create socket with SO_REUSEADDR to allow rebinding
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .map_err(|e| format!("Failed to create socket: {}", e))?;
        
        socket.set_reuse_address(true)
            .map_err(|e| format!("Failed to set reuse address: {}", e))?;
        
        let addr = "0.0.0.0:9999".parse::<std::net::SocketAddr>().unwrap();
        socket.bind(&addr.into())
            .map_err(|e| format!("Failed to bind: {}", e))?;
        
        let socket: UdpSocket = socket.into();
        
        socket.join_multicast_v4(
            &"239.0.0.1".parse::<Ipv4Addr>().unwrap(),
            &Ipv4Addr::UNSPECIFIED
        ).map_err(|e| format!("Failed to join multicast: {}", e))?;
        
        socket.set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .map_err(|e| format!("Failed to set timeout: {}", e))?;
        
        Ok(Self {
            socket: Arc::new(socket),
            is_running: Arc::new(Mutex::new(false)),
            frame_buffer: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    pub fn start_receiving(&self, app: AppHandle) -> Result<(), String> {
        *self.is_running.lock().unwrap() = true;
        let socket = self.socket.clone();
        let is_running = self.is_running.clone();
        let frame_buffer = self.frame_buffer.clone();
        
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 65535];
            
            while *is_running.lock().unwrap() {
                match socket.recv_from(&mut buf) {
                    Ok((size, _)) => {
                        if size < 12 { continue; }
                        
                        let frame_id = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                        let chunk_idx = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
                        let total_chunks = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
                        let chunk_data = buf[12..size].to_vec();
                        
                        let mut buffer = frame_buffer.lock().unwrap();
                        
                        // Clean up old incomplete frames
                        let now = std::time::Instant::now();
                        buffer.retain(|_, (_, timestamp)| {
                            now.duration_since(*timestamp).as_millis() < FRAME_TIMEOUT_MS as u128
                        });
                        
                        let (chunks, _) = buffer.entry(frame_id).or_insert_with(|| {
                            (vec![Vec::new(); total_chunks as usize], now)
                        });
                        
                        if (chunk_idx as usize) < chunks.len() {
                            chunks[chunk_idx as usize] = chunk_data;
                        }
                        
                        // Check if frame is complete
                        if chunks.iter().all(|c| !c.is_empty()) {
                            let complete_frame: Vec<u8> = chunks.concat();
                            let base64_image = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &complete_frame);
                            
                            let _ = app.emit("screen-frame", base64_image);
                            buffer.remove(&frame_id);
                        }
                    }
                    Err(_) => continue,
                }
            }
        });
        
        Ok(())
    }
    
    pub fn stop(&self) {
        *self.is_running.lock().unwrap() = false;
    }
}
