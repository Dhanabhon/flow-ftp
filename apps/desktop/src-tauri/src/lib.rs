mod bridge;
mod edit;
mod ipc;
mod profiles;
mod sync;
mod transcript;
mod transfers;

use bridge::ConnectionRegistry;
use tauri::{Emitter, Manager};
use transfers::{start_engine, CredentialCache};

/// Counts shipped to the frontend so its quit dialog can phrase the warning.
#[derive(Clone, serde::Serialize)]
struct QuitInfo {
    pending: usize,
    connected: usize,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    transcript::install();
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(target_os = "macos")]
    let builder = builder.menu(|app| {
        use tauri::menu::{AboutMetadata, MenuBuilder, SubmenuBuilder};

        let app_menu = SubmenuBuilder::new(app, "FlowFTP")
            .about_with_text(
                "About FlowFTP",
                Some(AboutMetadata {
                    name: Some("FlowFTP".into()),
                    version: Some(env!("CARGO_PKG_VERSION").into()),
                    short_version: Some(String::new()),
                    credits: Some("Powered by TOMERA Co., Ltd.".into()),
                    icon: Some(tauri::image::Image::from_bytes(include_bytes!(
                        "../icons/icon.png"
                    ))?),
                    ..Default::default()
                }),
            )
            .separator()
            .hide_with_text("Hide FlowFTP")
            .hide_others()
            .separator()
            .quit_with_text("Quit FlowFTP")
            .build()?;
        let file_menu = SubmenuBuilder::new(app, "File").close_window().build()?;
        let edit_menu = SubmenuBuilder::new(app, "Edit")
            .undo()
            .redo()
            .separator()
            .cut()
            .copy()
            .paste()
            .select_all()
            .build()?;
        let view_menu = SubmenuBuilder::new(app, "View").fullscreen().build()?;
        let window_menu = SubmenuBuilder::new(app, "Window")
            .minimize()
            .maximize()
            .separator()
            .close_window()
            .build()?;

        MenuBuilder::new(app)
            .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu])
            .build()
    });

    builder
        .on_window_event(|window, event| {
            // Quit confirmation: warn before killing in-flight transfers or
            // dropping live server sessions.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let engine = window
                    .app_handle()
                    .state::<std::sync::Arc<flow_transfer::TransferEngine>>()
                    .inner()
                    .clone();
                let pending = tauri::async_runtime::block_on(engine.pending_work_count());
                let connected = window
                    .app_handle()
                    .state::<ConnectionRegistry>()
                    .connected_count_now();
                if pending == 0 && connected == 0 {
                    return; // nothing at stake: close as usual
                }
                api.prevent_close();
                // The themed in-app dialog asks the user; the frontend calls
                // the confirm_quit command to finish the exit.
                let _ = window
                    .app_handle()
                    .emit("quit-confirm", QuitInfo { pending, connected });
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
            ipc::confirm_quit,
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
