import { useAppStore } from './store/appStore';
import { Sidebar } from './components/Sidebar';
import { MainView } from './components/MainView';
import { ScanOverlay } from './components/ScanOverlay';
import './index.css';
import { useKeyboard } from './hooks/useKeyboard';
import { useEffect } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { getCurrentWindow } from '@tauri-apps/api/window';

export default function App() {
  useKeyboard();
  const scanning = useAppStore(s => s.scanning);
  
  useEffect(() => {
    getVersion().then(version => {
      getCurrentWindow().setTitle(`Photo Selector v${version}`);
    }).catch(err => console.error('title error:', err));
  }, []);

  return (
    <div style={{
      display: 'flex',
      height: '100vh',
      width: '100vw',
      overflow: 'hidden',
      background: 'var(--bg-base)',
    }}>
      <Sidebar />
      <MainView />
      {scanning && <ScanOverlay />}
    </div>
  );
}
