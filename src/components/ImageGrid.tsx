import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { useState } from 'react';
import type { PageState, ImageInfo } from '../store/events';

interface Props {
  page: PageState;
}

export function ImageGrid({ page }: Props) {
  // Column count based on view_count
  const cols = page.view_count === 1 ? 1
             : page.view_count === 2 ? 2
             : page.view_count <= 4  ? 2   // 4 = 2×2
             : page.view_count <= 6  ? 3   // 6 = 2×3
             : 4;                          // 8 = 2×4

  return (
    <div style={{
      display: 'grid',
      gridTemplateColumns: `repeat(${cols}, 1fr)`,
      gap: 8,
      width: '100%',
      height: '100%',
      padding: 16,
      overflow: 'hidden',
    }}>
      {page.images.map((img, viewIndex) => (
        <ImageTile
          key={img.path}
          image={img}
          viewIndex={viewIndex}
        />
      ))}
    </div>
  );
}

function ImageTile({ image, viewIndex }: {
  image: ImageInfo;
  viewIndex: number;
}) {
  const [hovered, setHovered] = useState(false);
  const [acting,  setActing]  = useState(false);

  const src      = convertFileSrc(image.path);
  const filename = image.path.split('/').pop()
                ?? image.path.split('\\').pop()
                ?? image.path;
  const sizeKb   = image.file_size != null
                 ? `${(image.file_size / 1024).toFixed(0)} KB`
                 : '';

  async function handleAction(action: 'select' | 'reject') {
    if (acting) return;
    setActing(true);
    await invoke(
      action === 'select' ? 'select_image' : 'reject_image',
      { viewIndex }
    );
    setActing(false);
  }

  return (
    <div
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        position: 'relative',
        overflow: 'hidden',
        borderRadius: 4,
        background: 'var(--bg-raised)',
        // Fix: never mix border shorthand and borderColor — always set border fully
        border: hovered
          ? '1px solid var(--accent)'
          : '1px solid var(--border)',
        display: 'flex',
        flexDirection: 'column',
        cursor: 'default',
        transition: 'border-color var(--transition)',
      }}
    >
      {/* Image */}
      <div style={{
        flex: 1,
        overflow: 'hidden',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: 0,
      }}>
        <img
          src={src}
          alt={filename}
          style={{
            maxWidth: '100%',
            maxHeight: '100%',
            objectFit: 'contain',
            display: 'block',
            opacity: acting ? 0.4 : 1,
            transition: 'opacity var(--transition)',
          }}
        />
      </div>

      {/* Metadata bar */}
      <div style={{
        padding: '6px 10px',
        borderTop: '1px solid var(--border-subtle)',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        flexShrink: 0,
      }}>
        <span style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 10,
          color: 'var(--text-muted)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          maxWidth: '70%',
        }}>
          {filename}
        </span>
        <span style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 10,
          color: 'var(--text-muted)',
          flexShrink: 0,
        }}>
          {sizeKb}
        </span>
      </div>

      {/* Select / Reject buttons — visible on hover */}
      {hovered && !acting && (
        <div style={{
          position: 'absolute',
          bottom: 36,
          left: 0,
          right: 0,
          display: 'flex',
          gap: 8,
          padding: '0 10px',
          justifyContent: 'center',
        }}>
          <ActionBtn
            label="✓ Keep"
            bg="var(--select-green)"
            hover="var(--select-hover)"
            onClick={() => handleAction('select')}
          />
          <ActionBtn
            label="✕ Reject"
            bg="var(--reject-red)"
            hover="var(--reject-hover)"
            onClick={() => handleAction('reject')}
          />
        </div>
      )}
    </div>
  );
}

function ActionBtn({ label, bg, hover, onClick }: {
  label: string;
  bg: string;
  hover: string;
  onClick: () => void;
}) {
  const [h, setH] = useState(false);
  return (
    <button
      onClick={onClick}
      onMouseEnter={() => setH(true)}
      onMouseLeave={() => setH(false)}
      style={{
        padding: '7px 16px',
        borderRadius: 4,
        background: h ? hover : bg,
        color: '#fff',
        fontFamily: 'var(--font-ui)',
        fontSize: 12,
        fontWeight: 500,
        border: 'none',
        cursor: 'pointer',
        transition: 'background var(--transition)',
      }}
    >
      {label}
    </button>
  );
}