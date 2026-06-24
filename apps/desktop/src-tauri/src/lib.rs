mod commands;
mod state;

use commands::{
    cancel_scan, is_elevated, list_candidates, list_volumes, preview_candidate, recover_candidates,
    restart_elevated, start_scan,
};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_volumes,
            is_elevated,
            restart_elevated,
            start_scan,
            cancel_scan,
            list_candidates,
            preview_candidate,
            recover_candidates,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o PhotoRescue");
}
