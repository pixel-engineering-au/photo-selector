import { useAppStore } from '../store/appStore';

export function ScanOverlay() {
  const scanCount = useAppStore(s => s.scanCount);
  const loadedDir = useAppStore(s => s.loadedDir);

  // Show just the folder name, not the full path
  const dirName = loadedDir
    ? (loadedDir.split('/').pop() ?? loadedDir.split('\\').pop() ?? loadedDir)
    : '';

  return (
    <div style={{
      position: 'fixed',
      inset: 0,
      background: 'rgba(10,10,10,0.92)',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 16,
      zIndex: 100,
    }}>

      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--text-muted)',
        letterSpacing: '0.1em',
        textTransform: 'uppercase',
      }}>
        Scanning
      </div>

      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 64,
        fontWeight: 500,
        color: 'var(--accent)',
        lineHeight: 1,
        minWidth: 160,
        textAlign: 'center',
        animation: 'scan-pulse 1s ease-in-out infinite',
      }}>
        {scanCount}
      </div>

      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--text-muted)',
      }}>
        {dirName}
      </div>

      <style>{`
        @keyframes scan-pulse {
          0%, 100% { opacity: 1; }
          50%       { opacity: 0.5; }
        }
      `}</style>

    </div>
  );
}