import { invoke } from '@tauri-apps/api/core';
import type { PageState } from '../store/events';

interface Props {
  page: PageState;
}

export function NavBar({ page }: Props) {

  async function handlePrev() {
    await invoke('prev_page');
  }

  async function handleNext() {
    await invoke('next_page');
  }

  const isSinglePage = page.total_pages <= 1;
  const isFirst      = page.current_page === 0;
  const isLast       = page.current_page >= page.total_pages - 1;

  return (
    <div style={{
      height: 48,
      flexShrink: 0,
      borderTop: '1px solid var(--border)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '0 16px',
      background: 'var(--bg-surface)',
    }}>

      <NavButton
        onClick={handlePrev}
        disabled={isSinglePage || isFirst}
      >
        ← Prev
      </NavButton>

      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--text-muted)',
        letterSpacing: '0.08em',
      }}>
        {page.current_page + 1} / {page.total_pages}
        <span style={{ marginLeft: 12, opacity: 0.6 }}>
          {page.total} remaining
        </span>
      </div>

      <NavButton
        onClick={handleNext}
        disabled={isSinglePage || isLast}
      >
        Next →
      </NavButton>

    </div>
  );
}

function NavButton({ onClick, disabled, children }: {
  onClick: () => void;
  disabled: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        padding: '5px 14px',
        background: 'transparent',
        color: disabled ? 'var(--text-muted)' : 'var(--text-secondary)',
        borderRadius: 4,
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        border: '1px solid transparent',
        transition: 'var(--transition)',
        cursor: disabled ? 'not-allowed' : 'pointer',
      }}
    >
      {children}
    </button>
  );
}