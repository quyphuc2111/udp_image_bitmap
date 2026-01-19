mod screen_capture;
mod udp_server;
mod udp_client;

use tauri::State;
use std::sync::Mutex;
use serde::Serialize;

#[derive(Serialize)]
struct DisplayInfo {
    index: usize,
    width: usize,
    height: usize,
}

struct AppState {
    server: Mutex<Option<udp_server::UdpServer>>,
    client: Mutex<Option<udp_client::UdpClient>>,
}

#[tauri::command]
async fn start_server(state: State<'_, AppState>) -> Result<String, String> {
    let server = udp_server::UdpServer::new()?;
    server.start_streaming(screen_capture::capture_screen).await?;
    
    *state.server.lock().unwrap() = Some(server);
    Ok("Server started successfully".to_string())
}

#[tauri::command]
fn stop_server(state: State<'_, AppState>) -> Result<String, String> {
    if let Some(server) = state.server.lock().unwrap().as_ref() {
        server.stop();
    }
    *state.server.lock().unwrap() = None;
    Ok("Server stopped".to_string())
}

#[tauri::command]
fn start_client(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let client = udp_client::UdpClient::new()?;
    client.start_receiving(app)?;
    
    *state.client.lock().unwrap() = Some(client);
    Ok("Client started successfully".to_string())
}

#[tauri::command]
fn stop_client(state: State<'_, AppState>) -> Result<String, String> {
    if let Some(client) = state.client.lock().unwrap().as_ref() {
        client.stop();
    }
    *state.client.lock().unwrap() = None;
    Ok("Client stopped".to_string())
}

#[tauri::command]
fn get_displays() -> Result<Vec<DisplayInfo>, String> {
    let displays = screen_capture::get_displays()?;
    Ok(displays
        .into_iter()
        .map(|(index, width, height)| DisplayInfo { index, width, height })
        .collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            server: Mutex::new(None),
            client: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            start_server,
            stop_server,
            start_client,
            stop_client,
            get_displays
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
