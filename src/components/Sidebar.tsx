import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useAppStore } from '../store/appStore';

const SORT_OPTIONS = [
  { value: 'NameAsc',          label: 'Name A→Z' },
  { value: 'NameDesc',         label: 'Name Z→A' },
  { value: 'DateModifiedDesc', label: 'Newest first' },
  { value: 'DateModifiedAsc',  label: 'Oldest first' },
  { value: 'SizeDesc',         label: 'Largest first' },
  { value: 'SizeAsc',          label: 'Smallest first' },
];

const VIEW_OPTIONS = [1, 2, 4, 6, 8];

export function Sidebar() {
  const stats   = useAppStore(s => s.stats);
  const canUndo = useAppStore(s => s.canUndo);
  const page    = useAppStore(s => s.page);

  async function handleOpenFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === 'string') {
      await invoke('open_directory', { path: selected });
    }
  }

  async function handleUndo() {
    await invoke('undo_action');
  }

  // Wrap order in { type } to match serde(tag = "type") on SortOrder
  async function handleSort(order: string) {
    await invoke('set_sort_order', { order: { type: order } });
  }

  async function handleViewCount(count: number) {
    await invoke('set_view_count', { count });
  }

  const total    = stats ? stats.remaining + stats.selected + stats.rejected : 0;
  const actioned = stats ? stats.selected + stats.rejected : 0;
  const progress = total > 0 ? (actioned / total) * 100 : 0;

  return (
    <aside style={{
      width: 'var(--sidebar-width)',
      minWidth: 'var(--sidebar-width)',
      height: '100vh',
      background: 'var(--bg-surface)',
      borderRight: '1px solid var(--border)',
      display: 'flex',
      flexDirection: 'column',
      padding: '20px 0',
      overflowY: 'auto',
    }}>

      {/* App title */}
      <div style={{ padding: '0 20px 20px' }}>
        <div style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 11,
          letterSpacing: '0.15em',
          textTransform: 'uppercase',
          color: 'var(--accent)',
        }}>
          Photo Selector
        </div>
      </div>

      <Divider />

      {/* Open folder */}
      <Section>
        <ActionButton onClick={handleOpenFolder} primary>
          Open Folder
        </ActionButton>
      </Section>

      <Divider />

      {/* Stats */}
      {stats && (
        <>
          <Section label="Session">
            <StatRow label="Remaining" value={stats.remaining} />
            <StatRow label="Selected"  value={stats.selected}  accent="var(--select-green)" />
            <StatRow label="Rejected"  value={stats.rejected}  accent="var(--reject-red)" />
          </Section>

          {/* Progress bar */}
          <div style={{ padding: '0 20px 16px' }}>
            <div style={{
              height: 2,
              background: 'var(--bg-overlay)',
              borderRadius: 1,
              overflow: 'hidden',
            }}>
              <div style={{
                height: '100%',
                width: `${progress}%`,
                background: 'var(--accent)',
                transition: 'width 300ms ease',
                borderRadius: 1,
              }} />
            </div>
            <div style={{
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: 'var(--text-muted)',
              marginTop: 6,
              textAlign: 'right',
            }}>
              {Math.round(progress)}% complete
            </div>
          </div>

          <Divider />
        </>
      )}

      {/* Sort */}
      {page && (
        <>
          <Section label="Sort">
            <select
              onChange={e => handleSort(e.target.value)}
              style={{
                width: '100%',
                background: 'var(--accent)',
                border: '1px solid var(--accent-dim)',
                color: '#000',
                padding: '7px 10px',
                borderRadius: 4,
                fontFamily: 'var(--font-ui)',
                fontSize: 12,
                fontWeight: 500,
                cursor: 'pointer',
                appearance: 'none',
                WebkitAppearance: 'none',
              }}
            >
              {SORT_OPTIONS.map(o => (
                <option
                  key={o.value}
                  value={o.value}
                  style={{ background: '#1a1200', color: '#fff' }}
                >
                  {o.label}
                </option>
              ))}
            </select>
          </Section>

          <Divider />

          {/* View count */}
          <Section label="View">
            <div style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(5, 1fr)',
              gap: 4,
            }}>
              {VIEW_OPTIONS.map(n => (
                <button
                  key={n}
                  onClick={() => handleViewCount(n)}
                  style={{
                    padding: '6px 0',
                    background: page.view_count === n
                      ? 'var(--accent)' : 'var(--bg-overlay)',
                    color: page.view_count === n
                      ? '#000' : 'var(--text-secondary)',
                    borderRadius: 4,
                    fontFamily: 'var(--font-mono)',
                    fontSize: 11,
                    fontWeight: 500,
                    transition: 'var(--transition)',
                    border: '1px solid var(--border)',
                    cursor: 'pointer',
                  }}
                >
                  {n}
                </button>
              ))}
            </div>
          </Section>

          <Divider />
        </>
      )}

      {/* Undo */}
      <Section>
        <ActionButton onClick={handleUndo} disabled={!canUndo}>
          ↩ Undo last action
        </ActionButton>
      </Section>

    </aside>
  );
}

// ── Small helpers ─────────────────────────────────────────────────────────────

function Divider() {
  return (
    <div style={{
      height: 1,
      background: 'var(--border-subtle)',
      margin: '0 0 16px',
    }} />
  );
}

function Section({ label, children }: {
  label?: string;
  children: React.ReactNode;
}) {
  return (
    <div style={{ padding: '0 20px 16px' }}>
      {label && (
        <div style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 10,
          color: 'var(--text-muted)',
          letterSpacing: '0.12em',
          textTransform: 'uppercase',
          marginBottom: 8,
        }}>
          {label}
        </div>
      )}
      {children}
    </div>
  );
}

function StatRow({ label, value, accent }: {
  label: string;
  value: number;
  accent?: string;
}) {
  return (
    <div style={{
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
      padding: '3px 0',
    }}>
      <span style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 11,
        color: 'var(--text-secondary)',
      }}>
        {label}
      </span>
      <span style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 13,
        fontWeight: 500,
        color: accent ?? 'var(--text-primary)',
      }}>
        {value}
      </span>
    </div>
  );
}

function ActionButton({ onClick, children, primary, disabled }: {
  onClick: () => void;
  children: React.ReactNode;
  primary?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        width: '100%',
        padding: '8px 12px',
        background: primary ? 'var(--accent)' : 'var(--bg-overlay)',
        color: primary ? '#000' : 'var(--text-secondary)',
        borderRadius: 4,
        fontFamily: 'var(--font-ui)',
        fontSize: 12,
        fontWeight: primary ? 500 : 400,
        border: '1px solid var(--border)',
        transition: 'var(--transition)',
        textAlign: 'left',
        cursor: disabled ? 'not-allowed' : 'pointer',
      }}
    >
      {children}
    </button>
  );
}