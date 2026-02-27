import { useAppStore } from '../store/appStore';
import { ImageGrid } from './ImageGrid';
import { NavBar } from './NavBar';

export function MainView() {
  const page    = useAppStore(s => s.page);
  const isEmpty = useAppStore(s => s.isEmpty);

  return (
    <main style={{
      flex: 1,
      height: '100vh',
      display: 'flex',
      flexDirection: 'column',
      overflow: 'hidden',
      background: 'var(--bg-base)',
    }}>

      {/* Image display area */}
      <div style={{
        flex: 1,
        overflow: 'hidden',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}>
        {isEmpty && <EmptyState />}
        {!isEmpty && !page && <WelcomeState />}
        {page && <ImageGrid page={page} />}
      </div>

      {/* Navigation bar — only shown when images are loaded */}
      {page && <NavBar page={page} />}

    </main>
  );
}

function WelcomeState() {
  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: 12,
      userSelect: 'none',
    }}>
      <div style={{ fontSize: 48, opacity: 0.15 }}>⬜</div>
      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--text-muted)',
        letterSpacing: '0.12em',
        textTransform: 'uppercase',
      }}>
        Open a folder to begin
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: 12,
      userSelect: 'none',
    }}>
      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--accent)',
        letterSpacing: '0.12em',
        textTransform: 'uppercase',
      }}>
        All images processed
      </div>
      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--text-muted)',
      }}>
        Open a new folder to continue
      </div>
    </div>
  );
}