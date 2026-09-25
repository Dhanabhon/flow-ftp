mod bridge;
mod ipc;
mod transfers;

use bridge::ConnectionRegistry;
use tauri::Manager;
use transfers::{start_engine, CredentialCache};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(ConnectionRegistry::default())
        .manage(CredentialCache::default())
        .invoke_handler(tauri::generate_handler![
            ipc::remote_connect,
            ipc::remote_disconnect,
            ipc::remote_list,
            ipc::remote_stat,
            ipc::remote_mkdir,
            ipc::remote_rename,
            ipc::remote_delete,
            ipc::local_home,
            ipc::local_list,
            transfers::transfer_enqueue,
            transfers::transfer_list,
            transfers::transfer_pause,
            transfers::transfer_resume,
            transfers::transfer_cancel,
            transfers::transfer_clear_finished,
        ])
        .setup(|app| {
            // Transfer engine: event forwarding + worker pool.
            let engine = start_engine(app.handle().clone());
            app.manage(engine);
            // DevTools are force-enabled in debug builds by Tauri. Close them on
            // startup so they don't auto-open during development. Users who need
            // them can re-open via the app menu (when wired up) or keyboard
            // shortcut; production builds never ship devtools.
            #[cfg(debug_assertions)]
            {
                if let Some(win) = app.get_webview_window("main") {
                    win.close_devtools();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running FlowFTP application");
}
