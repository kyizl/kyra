#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod commands;
mod migration;
mod state;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::SIZE,
                )
                .build(),
        )
        .manage(state::AppState::default())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let app_config_dir = app.path().app_config_dir()?;
            if let Err(error) = migration::migrate_legacy_data(&app_data_dir, &app_config_dir) {
                eprintln!("legacy data migration incomplete: {error}");
            }
            let state = app.state::<state::AppState>();
            tauri::async_runtime::block_on(state.start_proxy(app.handle().clone()))
                .map_err(std::io::Error::other)?;
            if let Some(window) = app.get_webview_window("main") {
                window.set_always_on_top(true)?;
                window.set_ignore_cursor_events(false)?;
                window.set_decorations(false)?;
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::window_minimize,
            commands::window_close,
            commands::window_open_external,
            commands::clipboard_set_text,
            commands::updater_check,
            commands::updater_install,
            commands::auth_start,
            commands::auth_poll,
            commands::auth_status,
            commands::auth_refresh,
            commands::auth_logout,
            commands::rpc_set_enabled,
            commands::rpc_set_active,
            commands::rpc_set_network,
            commands::rpc_destroy,
            commands::stats_fetch,
            commands::stats_get,
            commands::stats_clan,
            commands::telemetry_is_linked,
            commands::telemetry_start_link,
            commands::support_list,
            commands::support_create,
            commands::support_get,
            commands::support_reply,
            commands::support_socket_connect,
            commands::support_socket_disconnect,
            commands::perf_dump,
            commands::perf_start_trace,
            commands::perf_stop_trace,
            commands::perf_open_log_dir,
            commands::shortcuts_register,
            commands::window_show,
            commands::window_focus,
            commands::window_toggle_minimize,
            commands::window_set_ignore_mouse,
            commands::cursor_poll_start,
            commands::cursor_poll_stop,
            commands::window_set_always_on_top,
            commands::window_screenshot,
            commands::window_fit_content_width,
            commands::proxy_get_status,
            commands::proxy_set_port,
            commands::proxy_set_bind_host,
            commands::proxy_start,
            commands::proxy_configure_and_start,
            commands::proxy_stop,
            commands::app_get_path,
            commands::app_read_file_base64,
            commands::app_open_image_dialog,
            commands::app_find_lunar_log,
            commands::log_check_path,
            commands::log_open_dialog,
            commands::log_set_path
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Kyra Overlay");
}
