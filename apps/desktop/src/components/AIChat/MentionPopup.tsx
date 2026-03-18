import { useState, useRef, useEffect, useCallback } from 'react';

export interface MentionOption {
  id: string;
  label: string;
  description: string;
  icon: React.ReactNode;
  prefix: string;
}

interface MentionPopupProps {
  visible: boolean;
  filter: string;
  position: { top: number; left: number };
  onSelect: (option: MentionOption) => void;
  onClose: () => void;
}

const styles = {
  container: {
    position: 'absolute' as const,
    zIndex: 200,
    background: '#252526',
    border: '1px solid #3c3c3c',
    borderRadius: 6,
    boxShadow: '0 8px 24px rgba(0, 0, 0, 0.4)',
    width: 280,
    maxHeight: 240,
    overflow: 'auto',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  header: {
    padding: '6px 10px',
    fontSize: 10,
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    letterSpacing: 0.5,
    color: '#6a6a6a',
    borderBottom: '1px solid #3c3c3c',
  } as React.CSSProperties,
  option: {
    display: 'flex',
    alignItems: 'center',
    padding: '8px 10px',
    cursor: 'pointer',
    gap: 10,
    transition: 'background 0.05s',
  } as React.CSSProperties,
  optionActive: {
    background: '#062f4a',
  } as React.CSSProperties,
  optionIcon: {
    width: 28,
    height: 28,
    borderRadius: 4,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
    fontSize: 14,
  } as React.CSSProperties,
  optionText: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 1,
    minWidth: 0,
  } as React.CSSProperties,
  optionLabel: {
    fontSize: 13,
    color: '#cccccc',
    fontWeight: 500,
  } as React.CSSProperties,
  optionDesc: {
    fontSize: 11,
    color: '#6a6a6a',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  } as React.CSSProperties,
  optionPrefix: {
    marginLeft: 'auto',
    fontSize: 11,
    color: '#969696',
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    flexShrink: 0,
  } as React.CSSProperties,
  empty: {
    padding: 16,
    textAlign: 'center' as const,
    color: '#6a6a6a',
    fontSize: 12,
  } as React.CSSProperties,
};

const MENTION_OPTIONS: MentionOption[] = [
  {
    id: 'file',
    label: 'File',
    description: 'Insert file content into context',
    prefix: '@file',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="#569cd6">
        <path d="M13.71 4.29l-3-3L10 1H4L3 2v12l1 1h9l1-1V5l-.29-.71zM13 14H4V2h5v4h4v8z" />
      </svg>
    ),
  },
  {
    id: 'codebase',
    label: 'Codebase',
    description: 'Search and reference codebase',
    prefix: '@codebase',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="#4ec9b0">
        <path d="M15.25 0a8.25 8.25 0 0 0-6.18 13.72L1 21.75l1.27 1.27 8.05-8.04A8.25 8.25 0 1 0 15.25 0zm0 14.5a6.25 6.25 0 1 1 0-12.5 6.25 6.25 0 0 1 0 12.5z" />
      </svg>
    ),
  },
  {
    id: 'git',
    label: 'Git',
    description: 'Show git status and changes',
    prefix: '@git',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="#ce9178">
        <path d="M21.007 8.222A3.738 3.738 0 0 0 15.045 5.2a3.737 3.737 0 0 0 1.156 6.583 2.988 2.988 0 0 1-2.668 1.67h-2.99a4.456 4.456 0 0 0-2.989 1.165V7.4a3.737 3.737 0 1 0-1.494 0v9.117a3.776 3.776 0 1 0 1.816.099 2.99 2.99 0 0 1 2.668-1.667h2.99a4.484 4.484 0 0 0 4.223-3.039 3.736 3.736 0 0 0 3.25-3.687z" />
      </svg>
    ),
  },
  {
    id: 'docs',
    label: 'Docs',
    description: 'Reference documentation',
    prefix: '@docs',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="#dcdcaa">
        <path d="M14.5 2h-13L1 2.5v11l.5.5h13l.5-.5v-11l-.5-.5zM14 13H2V6h12v7zm0-8H2V3h12v2z" />
      </svg>
    ),
  },
  {
    id: 'terminal',
    label: 'Terminal',
    description: 'Include terminal output',
    prefix: '@terminal',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="#c586c0">
        <path d="M1 2.795l.783-.419 5.371 3.581v.838l-5.371 3.581L1 9.957v-7.162zm0 8.205h14v1H1v-1z" />
      </svg>
    ),
  },
  {
    id: 'selection',
    label: 'Selection',
    description: 'Current editor selection',
    prefix: '@selection',
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="#b5cea8">
        <path d="M14 1H3L2 2v12l1 1h11l1-1V2l-1-1zM3 13V2h11v11H3z" />
      </svg>
    ),
  },
];

function getIconBg(id: string): string {
  switch (id) {
    case 'file': return '#569cd622';
    case 'codebase': return '#4ec9b022';
    case 'git': return '#ce917822';
    case 'docs': return '#dcdcaa22';
    case 'terminal': return '#c586c022';
    case 'selection': return '#b5cea822';
    default: return '#3c3c3c';
  }
}

export default function MentionPopup({
  visible,
  filter,
  position,
  onSelect,
  onClose,
}: MentionPopupProps) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);
  const optionRefs = useRef<(HTMLDivElement | null)[]>([]);

  // Filter options based on input
  const filteredOptions = MENTION_OPTIONS.filter((opt) => {
    if (!filter) return true;
    const lowerFilter = filter.toLowerCase();
    return (
      opt.label.toLowerCase().includes(lowerFilter) ||
      opt.prefix.toLowerCase().includes(lowerFilter) ||
      opt.description.toLowerCase().includes(lowerFilter)
    );
  });

  // Reset selection when filter changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [filter]);

  // Scroll selected into view
  useEffect(() => {
    optionRefs.current[selectedIndex]?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  // Keyboard navigation
  useEffect(() => {
    if (!visible) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          e.stopPropagation();
          setSelectedIndex((prev) => Math.min(prev + 1, filteredOptions.length - 1));
          break;
        case 'ArrowUp':
          e.preventDefault();
          e.stopPropagation();
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
          break;
        case 'Enter':
        case 'Tab':
          e.preventDefault();
          e.stopPropagation();
          if (filteredOptions[selectedIndex]) {
            onSelect(filteredOptions[selectedIndex]);
          }
          break;
        case 'Escape':
          e.preventDefault();
          onClose();
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [visible, selectedIndex, filteredOptions, onSelect, onClose]);

  // Close on outside click
  useEffect(() => {
    if (!visible) return;

    const handleClick = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [visible, onClose]);

  if (!visible) return null;

  return (
    <div
      ref={containerRef}
      style={{
        ...styles.container,
        top: position.top,
        left: position.left,
      }}
    >
      <div style={styles.header}>Mentions</div>
      {filteredOptions.length === 0 ? (
        <div style={styles.empty}>No matches found</div>
      ) : (
        filteredOptions.map((option, index) => (
          <div
            key={option.id}
            ref={(el) => { optionRefs.current[index] = el; }}
            style={{
              ...styles.option,
              ...(index === selectedIndex ? styles.optionActive : {}),
            }}
            onClick={() => onSelect(option)}
            onMouseEnter={() => setSelectedIndex(index)}
          >
            <div style={{ ...styles.optionIcon, background: getIconBg(option.id) }}>
              {option.icon}
            </div>
            <div style={styles.optionText}>
              <span style={styles.optionLabel}>{option.label}</span>
              <span style={styles.optionDesc}>{option.description}</span>
            </div>
            <span style={styles.optionPrefix}>{option.prefix}</span>
          </div>
        ))
      )}
    </div>
  );
}
