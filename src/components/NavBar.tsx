import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import type { PageState } from '../store/events';
import { useAppStore } from '../store/appStore';

interface Props {
  page: PageState;
}

export function NavBar({ page }: Props) {
  const [atFirst, setAtFirst] = useState(false);
  const [atLast,  setAtLast]  = useState(false);

  // Listen for boundary events to disable buttons
  // We do this via the store's applyEvent — extend the store
  // to expose atFirstPage / atLastPage if you want to track
  // boundaries. For now, reset on every PageChanged (which
  // means a successful navigation happened).

  async function handlePrev() {
    await invoke('prev_page');
  }

  async function handleNext() {
    await invoke('next_page');
  }

  const isSinglePage = page.total_pages <= 1;

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

      {/* Prev button */}
      <NavButton
        onClick={handlePrev}
        disabled={isSinglePage || page.current_page === 0}
      >
        ← Prev
      </NavButton>

      {/* Page indicator */}
      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--text-muted)',
        letterSpacing: '0.08em',
      }}>
        {page.current_page + 1} / {page.total_pages}
        <span style={{
          marginLeft: 12,
          color: 'var(--text-muted)',
          opacity: 0.6,
        }}>
          {page.total} remaining
        </span>
      </div>

      {/* Next button */}
      <NavButton
        onClick={handleNext}
        disabled={isSinglePage || page.current_page >= page.total_pages - 1}
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
