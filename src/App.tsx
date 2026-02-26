import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { startCoreListener  } from './store/events';
import type { AppEvent, PageState, LibraryStats } from './store/events';

function App() {
  const [events, setEvents]       = useState<string[]>([]);
  const [page, setPage]           = useState<PageState | null>(null);
  const [stats, setStats]         = useState<LibraryStats | null>(null);
  const [scanning, setScanning]   = useState(false);
  const [scanCount, setScanCount] = useState(0);

  useEffect(() => {
    const unlisten = startCoreListener((event: AppEvent) => {
      // Log every event to the debug list
      setEvents(prev => [...prev.slice(-20), JSON.stringify(event)]);

      switch (event.type) {
        case 'ScanStarted':
          setScanning(true);
          setScanCount(0);
          break;
        case 'ScanProgress':
          setScanCount(event.scanned);
          break;
        case 'ScanComplete':
          setScanning(false);
          break;
        case 'PageChanged':
          setPage(event.payload);
          break;
        case 'StatsChanged':
          setStats(event.payload);
          break;
      }
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  async function handleOpenFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === 'string') {
      await invoke('open_directory', { path: selected });
    }
  }

  return (
    <div style={{ fontFamily: 'monospace', padding: 20 }}>
      <h1>Photo Selector — Bridge Test</h1>

      <button onClick={handleOpenFolder}>Open Folder</button>

      {scanning && <p>Scanning... {scanCount} images found</p>}

      {stats && (
        <p>
          Remaining: {stats.remaining} |
          Selected: {stats.selected} |
          Rejected: {stats.rejected}
        </p>
      )}

      {page && (
        <div>
          <p>Page {page.current_page + 1} of {page.total_pages} ({page.total} images)</p>
          <ul>
            {page.images.map((img, i) => (
              <li key={i}>{img.path} — {img.file_size ?? '?'} bytes</li>
            ))}
          </ul>
          <button onClick={() => invoke('prev_page')}>← Prev</button>
          <button onClick={() => invoke('next_page')}>Next →</button>
        </div>
      )}

      <hr />
      <h3>Event log (last 20)</h3>
      <pre style={{ fontSize: 11, maxHeight: 300, overflow: 'auto' }}>
        {events.map((e, i) => <div key={i}>{e}</div>)}
      </pre>
    </div>
  );
}

export default App;