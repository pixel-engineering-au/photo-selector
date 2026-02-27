import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { startCoreListener } from './store/events';
import { useAppStore } from './store/appStore';
import './index.css';

// Start the core event listener once at app boot,
// outside the React tree so it never re-registers.
startCoreListener((event) => {
  useAppStore.getState().applyEvent(event);
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);