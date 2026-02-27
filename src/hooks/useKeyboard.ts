import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../store/appStore';

export function useKeyboard() {
  const page = useAppStore(s => s.page);

  useEffect(() => {
    async function handleKey(e: KeyboardEvent) {
      // Don't fire if focus is inside an input or select
      if (e.target instanceof HTMLInputElement
       || e.target instanceof HTMLSelectElement) return;

      switch (e.key) {
        case 'ArrowRight':
        case ' ':
          e.preventDefault();
          await invoke('next_page');
          break;
        case 'ArrowLeft':
          e.preventDefault();
          await invoke('prev_page');
          break;
        case 'z':
        case 'Z':
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            await invoke('undo_action');
          }
          break;
        // Single image shortcuts — only in 1-up view
        case 's':
        case 'S':
          if (page?.view_count === 1) {
            await invoke('select_image', { viewIndex: 0 });
          }
          break;
        case 'r':
        case 'R':
          if (page?.view_count === 1) {
            await invoke('reject_image', { viewIndex: 0 });
          }
          break;
      }
    }

    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [page]);
}