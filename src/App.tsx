import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useAppStore } from './store/appStore';
import { Sidebar } from './components/Sidebar';
import { MainView } from './components/MainView';
import { ScanOverlay } from './components/ScanOverlay';
import './index.css';
import { useKeyboard } from './hooks/useKeyboard';

export default function App() {
  useKeyboard();
  const scanning = useAppStore(s => s.scanning);

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
