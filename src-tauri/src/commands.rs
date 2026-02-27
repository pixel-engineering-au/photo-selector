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
    let events = state.0.lock()
        .map_err(|e| e.to_string())?
        .load_dir(&PathBuf::from(path));

    // Grab total from ScanComplete so we know when we're at the last progress event
    let total = events.iter().find_map(|e| {
        if let photo_selector_core::events::AppEvent::ScanComplete { total } = e {
            Some(*total)
        } else {
            None
        }
    }).unwrap_or(0);

    for event in events {
        if let photo_selector_core::events::AppEvent::ScanProgress { scanned } = &event {
            // Always emit first, last, and every 10th — never skip all of them
            let is_first = *scanned == 1;
            let is_last  = *scanned == total;
            let is_tenth = scanned % 10 == 0;
            if !is_first && !is_last && !is_tenth {
                continue;
            }
        }
        handle.emit("core-event", &event).ok();
    }
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