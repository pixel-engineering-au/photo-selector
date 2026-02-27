import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { startCoreListener } from './store/events';
import { useAppStore } from './store/appStore';
import './index.css';

// Mount React first — never let the listener block rendering
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// Start listener after React is mounted
startCoreListener((event) => {
  useAppStore.getState().applyEvent(event);
}).catch((err) => {
  console.error('Failed to start core listener:', err);
});