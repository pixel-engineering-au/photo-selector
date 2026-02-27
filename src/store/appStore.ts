import { create } from 'zustand';
import type { PageState, LibraryStats, AppEvent } from './events';

interface AppStore {
  // Core state
  page:       PageState | null;
  stats:      LibraryStats | null;
  scanning:   boolean;
  scanCount:  number;
  isEmpty:    boolean;
  canUndo:    boolean;

  // UI state
  loadedDir:  string | null;

  // Mutators — called by the event listener
  applyEvent: (event: AppEvent) => void;
}

export const useAppStore = create<AppStore>((set, get) => ({
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
        set({ scanning: true, scanCount: 0, isEmpty: false,
              canUndo: false, loadedDir: event.path });
        break;
      case 'ScanProgress':
        set({ scanCount: event.scanned });
        break;
      case 'ScanComplete':
        set({ scanning: false });
        break;
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
      // by the PageChanged and StatsChanged events that follow them
      default:
        break;
    }
  },
}));
