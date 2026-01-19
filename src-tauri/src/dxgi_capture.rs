// DXGI Desktop Duplication API for Windows
// Much faster than GDI/scrap for screen capture
// Based on RustDesk implementation but simplified for LAN

#[cfg(windows)]
use windows::Win32::{
    Graphics::{
        Direct3D11::*,
        Dxgi::{Common::*, *},
        Gdi::*,
    },
    Foundation::*,
    System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
};
#[cfg(windows)]
use std::ptr;

#[cfg(windows)]
pub struct DxgiCapturer {
    device: Option<ID3D11Device>,
    context: Option<ID3D11DeviceContext>,
    duplication: Option<IDXGIOutputDuplication>,
    width: usize,
    height: usize,
    timeout_ms: u32,
}

#[cfg(windows)]
impl DxgiCapturer {
    pub fn new(display_index: usize) -> Result<Self, String> {
        unsafe {
            // 1. Create DXGI factory
            let factory: IDXGIFactory1 = CreateDXGIFactory1()
                .map_err(|e| format!("Failed to create DXGI factory: {:?}", e))?;

            // 2. Get adapter (GPU)
            let adapter = factory.EnumAdapters1(0)
                .map_err(|e| format!("Failed to enumerate adapters: {:?}", e))?;

            // 3. Get output (monitor)
            let output = adapter.EnumOutputs(display_index as u32)
                .map_err(|e| format!("Failed to get output {}: {:?}", display_index, e))?;

            let output1: IDXGIOutput1 = output.cast()
                .map_err(|e| format!("Failed to cast to IDXGIOutput1: {:?}", e))?;

            // 4. Get output description
            let desc = output.GetDesc()
                .map_err(|e| format!("Failed to get output desc: {:?}", e))?;
            
            let width = (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left) as usize;
            let height = (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top) as usize;

            eprintln!("🖥️  DXGI Display {}: {}x{}", display_index, width, height);

            // 5. Create D3D11 device
            let mut device: Option<ID3D11Device> = None;
            let mut context: Option<ID3D11DeviceContext> = None;
            let mut feature_level = D3D_FEATURE_LEVEL_11_0;

            let hr = D3D11CreateDevice(
                &adapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&[
                    D3D_FEATURE_LEVEL_11_1,
                    D3D_FEATURE_LEVEL_11_0,
                    D3D_FEATURE_LEVEL_10_1,
                    D3D_FEATURE_LEVEL_10_0,
                ]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                Some(&mut feature_level),
                Some(&mut context),
            );

            if hr.is_err() {
                return Err(format!("Failed to create D3D11 device: {:?}", hr));
            }

            let device = device.ok_or("Device is None")?;
            let context = context.ok_or("Context is None")?;

            eprintln!("✅ D3D11 device created, feature level: {:?}", feature_level);

            // 6. Create output duplication
            let duplication = output1.DuplicateOutput(&device)
                .map_err(|e| format!("Failed to create output duplication: {:?}\n\
                    This may happen if:\n\
                    - Running in RDP session\n\
                    - No display attached\n\
                    - Another app is using duplication", e))?;

            eprintln!("✅ DXGI Output Duplication created successfully");

            Ok(Self {
                device: Some(device),
                context: Some(context),
                duplication: Some(duplication),
                width,
                height,
                timeout_ms: 100,
            })
        }
    }

    pub fn capture_frame(&mut self) -> Result<Vec<u8>, String> {
        unsafe {
            let duplication = self.duplication.as_ref()
                .ok_or("Duplication not initialized")?;
            let device = self.device.as_ref()
                .ok_or("Device not initialized")?;
            let context = self.context.as_ref()
                .ok_or("Context not initialized")?;

            // 1. Acquire next frame
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut desktop_resource: Option<IDXGIResource> = None;

            let result = duplication.AcquireNextFrame(
                self.timeout_ms,
                &mut frame_info,
                &mut desktop_resource,
            );

            match result {
                Ok(_) => {
                    // Got a new frame
                }
                Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => {
                    // No new frame yet, return WouldBlock
                    return Err("WouldBlock".to_string());
                }
                Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => {
                    // Display mode changed, need to recreate duplication
                    return Err("AccessLost - display changed".to_string());
                }
                Err(e) => {
                    return Err(format!("AcquireNextFrame failed: {:?}", e));
                }
            }

            let desktop_resource = desktop_resource
                .ok_or("Desktop resource is None")?;

            // 2. Get texture from resource
            let texture: ID3D11Texture2D = desktop_resource.cast()
                .map_err(|e| format!("Failed to cast to texture: {:?}", e))?;

            // 3. Create staging texture to read data
            let mut texture_desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut texture_desc);

            texture_desc.Usage = D3D11_USAGE_STAGING;
            texture_desc.BindFlags = D3D11_BIND_FLAG(0);
            texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
            texture_desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0);

            let staging_texture = device.CreateTexture2D(&texture_desc, None)
                .map_err(|e| format!("Failed to create staging texture: {:?}", e))?;

            // 4. Copy texture to staging
            context.CopyResource(&staging_texture, &texture);

            // 5. Map staging texture to read pixels
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            context.Map(
                &staging_texture,
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut mapped),
            ).map_err(|e| format!("Failed to map texture: {:?}", e))?;

            // 6. Convert BGRA to RGBA
            let row_pitch = mapped.RowPitch as usize;
            let src_data = std::slice::from_raw_parts(
                mapped.pData as *const u8,
                row_pitch * self.height,
            );

            let mut rgba_data = Vec::with_capacity(self.width * self.height * 4);
            
            for y in 0..self.height {
                let row_start = y * row_pitch;
                for x in 0..self.width {
                    let pixel_start = row_start + x * 4;
                    if pixel_start + 3 < src_data.len() {
                        // BGRA → RGBA
                        rgba_data.push(src_data[pixel_start + 2]); // R
                        rgba_data.push(src_data[pixel_start + 1]); // G
                        rgba_data.push(src_data[pixel_start]);     // B
                        rgba_data.push(src_data[pixel_start + 3]); // A
                    }
                }
            }

            // 7. Cleanup
            context.Unmap(&staging_texture, 0);
            duplication.ReleaseFrame()
                .map_err(|e| format!("Failed to release frame: {:?}", e))?;

            Ok(rgba_data)
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

#[cfg(windows)]
impl Drop for DxgiCapturer {
    fn drop(&mut self) {
        eprintln!("🔻 Dropping DXGI capturer");
    }
}

// Public API for cross-platform compatibility
#[cfg(windows)]
pub fn create_dxgi_capturer(display_index: usize) -> Result<DxgiCapturer, String> {
    DxgiCapturer::new(display_index)
}

#[cfg(not(windows))]
pub fn create_dxgi_capturer(_display_index: usize) -> Result<(), String> {
    Err("DXGI capture is Windows-only".to_string())
}

// Test if DXGI is available
#[cfg(windows)]
pub fn is_dxgi_available() -> bool {
    unsafe {
        windows::Win32::Graphics::Dxgi::CreateDXGIFactory1::<IDXGIFactory1>().is_ok()
    }
}

#[cfg(not(windows))]
pub fn is_dxgi_available() -> bool {
    false
}
