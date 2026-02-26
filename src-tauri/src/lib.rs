use tauri_plugin_dialog;
use tauri_plugin_shell;
mod state;
mod commands;

use state::TauriAppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(TauriAppState::new())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![
      commands::open_directory,
      commands::next_page,
      commands::prev_page,
      commands::select_image,
      commands::reject_image,
      commands::undo_action,
      commands::set_sort_order,
      commands::set_view_count,
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
