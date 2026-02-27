import { create } from 'zustand';
import type { PageState, LibraryStats, AppEvent } from './events';

interface AppStore {
  page:       PageState | null;
  stats:      LibraryStats | null;
  scanning:   boolean;
  scanCount:  number;
  isEmpty:    boolean;
  canUndo:    boolean;
  loadedDir:  string | null;
  applyEvent: (event: AppEvent) => void;
}

// Minimum ms the scan overlay stays visible so the user always sees it
const MIN_SCAN_DISPLAY_MS = 600;
let scanStartedAt  = 0;
let scanCompleteTimer: ReturnType<typeof setTimeout> | null = null;

export const useAppStore = create<AppStore>((set) => ({
  page:      null,
  stats:     null,
  scanning:  false,
  scanCount: 0,
  isEmpty:   false,
  canUndo:   false,
  loadedDir: null,

  applyEvent: (event: AppEvent) => {
    switch (event.type) {

      case 'ScanStarted':
        // Clear any timer left over from a previous scan
        if (scanCompleteTimer) {
          clearTimeout(scanCompleteTimer);
          scanCompleteTimer = null;
        }
        scanStartedAt = Date.now();
        set({
          scanning:  true,
          scanCount: 0,
          isEmpty:   false,
          canUndo:   false,
          loadedDir: event.payload.path,
        });
        break;

      case 'ScanProgress':
        set({ scanCount: event.payload.scanned });
        break;

      case 'ScanComplete': {
        // Always show the final count regardless of throttling
        set({ scanCount: event.payload.total });
        // Keep overlay visible for at least MIN_SCAN_DISPLAY_MS
        const elapsed   = Date.now() - scanStartedAt;
        const remaining = Math.max(0, MIN_SCAN_DISPLAY_MS - elapsed);
        scanCompleteTimer = setTimeout(() => {
          set({ scanning: false });
          scanCompleteTimer = null;
        }, remaining);
        break;
      }

      case 'PageChanged':
        set({ page: event.payload, isEmpty: false });
        break;

      case 'StatsChanged':
        set({ stats: event.payload });
        break;

      case 'LibraryEmpty':
        set({ page: null, isEmpty: true });
        break;

      case 'FileMoved':
        set({ canUndo: true });
        break;

      case 'Undone':
        break;

      case 'UndoStackEmpty':
        set({ canUndo: false });
        break;

      // NavigationBoundary, SortChanged, ViewCountChanged,
      // StaleEntryRemoved, DirectoryLoaded are handled implicitly
      // by the PageChanged and StatsChanged events that follow them.
      default:
        break;
    }
  },
}));