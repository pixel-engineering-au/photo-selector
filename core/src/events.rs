use std::path::PathBuf;
use crate::image_cache::Image;

/// A snapshot of the current page, emitted whenever visible contents change.
#[derive(Debug, Clone)]
pub struct PageState {
    /// Images currently visible in the page window.
    pub images: Vec<Image>,
    /// Absolute index into the full image list where this page starts.
    pub current_index: usize,
    /// Total images remaining in the library (after any removals).
    pub total: usize,
    /// How many pages exist at this view_count.
    pub total_pages: usize,
    /// Which page we are on (0-based).
    pub current_page: usize,
}

/// Every state change the core can emit.
/// Consumers (CLI, Tauri) match on these — no polling required.
///
/// Tauri note: add `#[derive(serde::Serialize, serde::Deserialize)]`
/// here (and to PageState / Image) when you add the tauri dependency.
/// The variants map directly to JSON objects emitted on the event bus.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// A directory was scanned and the library is ready.
    /// Always followed by a PageChanged if any images were found.
    DirectoryLoaded {
        path: PathBuf,
        total: usize,
    },

    /// The visible page changed — redraw the image grid.
    /// Emitted after: load_dir, next, prev, act_on_current_at, stale cleanup.
    PageChanged(PageState),

    /// A file was successfully moved to selected/ or rejected/.
    FileMoved {
        from: PathBuf,
        to: PathBuf,
        action: MoveAction,
    },

    /// A stale index entry was silently removed (file no longer on disk).
    StaleEntryRemoved {
        path: PathBuf,
    },

    /// The library has no more images — show empty state in GUI.
    LibraryEmpty,

    /// Navigation hit a boundary — useful for disabling prev/next buttons.
    NavigationBoundary {
        kind: BoundaryKind,
    },
}

/// Whether a file was selected or rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveAction {
    Select,
    Reject,
}

/// Which boundary was hit during navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryKind {
    /// Already on first page, prev() was a no-op.
    FirstPage,
    /// Already on last page, next() was a no-op.
    LastPage,
}