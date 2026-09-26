mod bridge;
mod edit;
mod ipc;
mod profiles;
mod sync;
mod transfers;

use bridge::ConnectionRegistry;
use tauri::Manager;
use transfers::{start_engine, CredentialCache};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            // Quit confirmation: warn before killing in-flight transfers.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let engine = window
                    .app_handle()
                    .state::<std::sync::Arc<flow_transfer::TransferEngine>>()
                    .inner()
                    .clone();
                let pending = tauri::async_runtime::block_on(engine.pending_work_count());
                if pending == 0 {
                    return; // nothing at stake: close as usual
                }
                api.prevent_close();

                let handle = window.app_handle().clone();
                let window = window.clone();
                tauri::async_runtime::spawn(async move {
                    use tauri_plugin_dialog::{
                        DialogExt, MessageDialogButtons, MessageDialogKind,
                    };
                    let plural = if pending == 1 { "" } else { "s" };
                    let quit = handle
                        .dialog()
                        .message(format!(
                            "{pending} transfer{plural} are still running or queued. \
                             Quitting now stops them; resumable files keep their progress."
                        ))
                        .title("Quit FlowFTP?")
                        .kind(MessageDialogKind::Warning)
                        .buttons(MessageDialogButtons::OkCancelCustom("Quit".into(), "Cancel".into()))
                        .blocking_show();
                    if quit {
                        let _ = window.destroy();
                    }
                });
            }
        })
        .manage(ConnectionRegistry::default())
        .manage(CredentialCache::default())
        .manage(edit::EditSessions::default())
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
            ipc::local_mkdir,
            ipc::local_rename,
            ipc::local_delete,
            ipc::local_read_text,
            transfers::transfer_enqueue,
            transfers::transfer_list,
            transfers::transfer_pause,
            transfers::transfer_resume,
            transfers::transfer_cancel,
            transfers::transfer_clear_finished,
            ipc::transfer_set_rate_limit,
            profiles::profile_list,
            profiles::profile_save,
            profiles::profile_delete,
            sync::sync_preview,
            sync::sync_execute,
            edit::remote_edit_open,
        ])
        .setup(|app| {
            // Saved connection profiles from the app config dir.
            let config_dir = app
                .path()
                .app_config_dir()
                .expect("app config dir must be resolvable");
            std::fs::create_dir_all(&config_dir).ok();
            app.manage(profiles::ProfileStore::load(&config_dir));

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
