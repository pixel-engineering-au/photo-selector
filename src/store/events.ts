import { listen } from '@tauri-apps/api/event';
import type {UnlistenFn } from '@tauri-apps/api/event';
// Every variant is wrapped in `payload` because the Rust side uses
// serde(tag = "type", content = "payload") on AppEvent.
// Unit variants (no fields) are the exception — they serialize as
// just { "type": "UndoStackEmpty" } with no payload key.
export type AppEvent =
  | { type: 'ScanStarted';        payload: { path: string } }
  | { type: 'ScanProgress';       payload: { scanned: number } }
  | { type: 'ScanComplete';       payload: { total: number } }
  | { type: 'DirectoryLoaded';    payload: { path: string; total: number } }
  | { type: 'PageChanged';        payload: PageState }
  | { type: 'FileMoved';          payload: { from: string; to: string; action: MoveAction } }
  | { type: 'Undone';             payload: { path: string; action: MoveAction } }
  | { type: 'UndoStackEmpty' }
  | { type: 'StatsChanged';       payload: LibraryStats }
  | { type: 'SortChanged';        payload: { order: SortOrder } }
  | { type: 'ViewCountChanged';   payload: { view_count: number } }
  | { type: 'StaleEntryRemoved';  payload: { path: string } }
  | { type: 'LibraryEmpty' }
  | { type: 'NavigationBoundary'; payload: { kind: BoundaryKind } };

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
  date_taken:  number | null;
  load_state:  LoadState;
}

export type LoadState =
  | { type: 'Pending' }
  | { type: 'Ready';  thumbnail: number[] }
  | { type: 'Failed'; reason: string };

export interface LibraryStats {
  remaining: number;
  selected:  number;
  rejected:  number;
}

export type MoveAction   = { type: 'Select' } | { type: 'Reject' };
export type BoundaryKind = { type: 'FirstPage' } | { type: 'LastPage' };
export type SortOrder    =
  | 'NameAsc' | 'NameDesc'
  | 'DateModifiedAsc' | 'DateModifiedDesc'
  | 'SizeAsc' | 'SizeDesc';

/// Start listening for all core events.
/// Call once at app startup. Returns an unlisten function to clean up.
export async function startCoreListener(
  onEvent: (event: AppEvent) => void
): Promise<UnlistenFn> {
  return listen<AppEvent>('core-event', (event) => {
    onEvent(event.payload);
  });
}