use std::path::PathBuf;
use crate::image_cache::Image;
use crate::image_index::SortOrder;
use crate::stats::LibraryStats;

/// A snapshot of the current page, emitted whenever visible contents change.
///
/// Tauri note: add `#[derive(serde::Serialize, serde::Deserialize)]`
/// here and on all referenced types when wiring up Tauri.
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
    /// How many images are visible per page.
    pub view_count: usize,
}

/// Every state change the core can emit.
/// Consumers (CLI, Tauri) match on these — no polling required.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Emitted once at the start of a directory scan.
    /// Consumers can use this to show a progress bar or spinner.
    ///
    /// Tauri note: emit this immediately on the event bus before the blocking
    /// scan runs so the frontend can show a loading state right away.
    ScanStarted {
        path: PathBuf,
    },

    /// Emitted once per discovered image during scanning.
    /// `scanned` is the running count of supported images found so far.
    /// There is no `total` because directory size is not known up front.
    ///
    /// Tauri: forward to the frontend to animate a live counter
    /// ("Scanning... 142 images found"). Throttle in the Tauri layer
    /// if needed (e.g. only forward every 10th event for large dirs).
    ScanProgress {
        scanned: usize,
    },

    /// Emitted once when scanning is complete and the index is fully built.
    /// Always followed by DirectoryLoaded, then PageChanged or LibraryEmpty.
    ScanComplete {
        total: usize,
    },

    /// A directory was scanned and the library is ready.
    /// Always followed by PageChanged (or LibraryEmpty) and StatsChanged.
    DirectoryLoaded {
        path: PathBuf,
        total: usize,
    },

    /// The visible page changed — redraw the image grid.
    /// Emitted after: load_dir, next, prev, act_on_current_at,
    /// undo, set_view_count, set_sort_order, stale cleanup.
    PageChanged(PageState),

    /// A file was successfully moved to selected/ or rejected/.
    FileMoved {
        from: PathBuf,
        to: PathBuf,
        action: MoveAction,
    },

    /// The last action was successfully undone — file is back in the library.
    Undone {
        /// The file's restored path (back in original directory).
        path: PathBuf,
        /// What action was undone.
        action: MoveAction,
    },

    /// Undo was requested but the stack was empty.
    UndoStackEmpty,

    /// Library stats changed — update counters in the sidebar.
    StatsChanged(LibraryStats),

    /// Sort order changed — the grid has been re-ordered.
    SortChanged {
        order: SortOrder,
    },

    /// View count (images per page) changed.
    ViewCountChanged {
        view_count: usize,
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