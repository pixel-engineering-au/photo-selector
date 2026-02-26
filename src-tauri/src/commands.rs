use std::path::PathBuf;
use tauri::{AppHandle,Emitter, State};
use photo_selector_core::app_state::{Action};
use photo_selector_core::image_index::SortOrder;
use crate::state::TauriAppState;

/// Emit all events from a core method to the frontend.
/// The frontend listens for "core-event" and routes by variant.
fn emit_all(handle: &AppHandle, events: Vec<photo_selector_core::events::AppEvent>) {
    for event in events {
        handle.emit("core-event", &event).ok();
    }
}

#[tauri::command]
pub fn open_directory(
    path: String,
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    // Synchronous scan is fine for now — ScanStarted/ScanProgress events
    // keep the frontend informed. Move to spawn_blocking when dirs exceed
    // ~10,000 images and the brief UI pause becomes noticeable.
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .load_dir(&PathBuf::from(path));
    emit_all(&handle, events);
    Ok(())
}

#[tauri::command]
pub fn next_page(
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .next();
    emit_all(&handle, events);
    Ok(())
}

#[tauri::command]
pub fn prev_page(
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .prev();
    emit_all(&handle, events);
    Ok(())
}

#[tauri::command]
pub fn select_image(
    view_index: usize,
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .act_on_current_at(Action::Select, view_index)
        .map_err(|e| e.to_string())?;
    emit_all(&handle, events);
    Ok(())
}

#[tauri::command]
pub fn reject_image(
    view_index: usize,
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .act_on_current_at(Action::Reject, view_index)
        .map_err(|e| e.to_string())?;
    emit_all(&handle, events);
    Ok(())
}

#[tauri::command]
pub fn undo_action(
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .undo()
        .map_err(|e| e.to_string())?;
    emit_all(&handle, events);
    Ok(())
}

#[tauri::command]
pub fn set_sort_order(
    order: SortOrder,
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .set_sort_order(order);
    emit_all(&handle, events);
    Ok(())
}

#[tauri::command]
pub fn set_view_count(
    count: usize,
    state: State<'_, TauriAppState>,
    handle: AppHandle,
) -> Result<(), String> {
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .set_view_count(count);
    emit_all(&handle, events);
    Ok(())
}