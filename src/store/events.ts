import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

// Mirror the Rust AppEvent enum variants as a TypeScript union.
// Add new variants here whenever the core adds them.
export type AppEvent =
  | { type: 'ScanStarted';   path: string }
  | { type: 'ScanProgress';  scanned: number }
  | { type: 'ScanComplete';  total: number }
  | { type: 'DirectoryLoaded'; path: string; total: number }
  | { type: 'PageChanged';   payload: PageState }
  | { type: 'FileMoved';     from: string; to: string; action: MoveAction }
  | { type: 'Undone';        path: string; action: MoveAction }
  | { type: 'UndoStackEmpty' }
  | { type: 'StatsChanged';  payload: LibraryStats }
  | { type: 'SortChanged';   order: SortOrder }
  | { type: 'ViewCountChanged'; view_count: number }
  | { type: 'StaleEntryRemoved'; path: string }
  | { type: 'LibraryEmpty' }
  | { type: 'NavigationBoundary'; kind: 'FirstPage' | 'LastPage' };

export interface PageState {
  images:        ImageInfo[];
  current_index: number;
  total:         number;
  total_pages:   number;
  current_page:  number;
  view_count:    number;
}

export interface ImageInfo {
  path:        string;
  dimensions:  [number, number] | null;
  file_size:   number | null;
  date_taken:  string | null;   // ISO string or null
  load_state:  LoadState;
}

export type LoadState =
  | { type: 'Pending' }
  | { type: 'Ready'; thumbnail: number[] }
  | { type: 'Failed'; reason: string };

export interface LibraryStats {
  remaining: number;
  selected:  number;
  rejected:  number;
}

export type MoveAction = 'Select' | 'Reject';
export type SortOrder  =
  | 'NameAsc' | 'NameDesc'
  | 'DateModifiedAsc' | 'DateModifiedDesc'
  | 'SizeAsc' | 'SizeDesc';

/// Start listening for all core events.
/// Call once at app startup; call the returned function to stop.
export async function startCoreListener(
  onEvent: (event: AppEvent) => void
): Promise<UnlistenFn> {
  return listen<AppEvent>('core-event', (event) => {
    onEvent(event.payload);
  });
}