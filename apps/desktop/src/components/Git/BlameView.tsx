import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface BlameAnnotation {
  lineNumber: number;
  hash: string;
  shortHash: string;
  author: string;
  date: string;
  relativeDate: string;
  message: string;
  lineContent: string;
}

interface BlameViewProps {
  filePath: string;
  projectPath: string;
}

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    height: '100%',
    background: 'var(--bg-primary)',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    overflow: 'auto',
  } as React.CSSProperties,
  header: {
    display: 'flex',
    alignItems: 'center',
    padding: '8px 12px',
    background: 'var(--bg-secondary)',
    borderBottom: '1px solid var(--border-color)',
    fontSize: 12,
    color: 'var(--text-secondary)',
    gap: 8,
    flexShrink: 0,
  } as React.CSSProperties,
  headerTitle: {
    color: 'var(--text-primary)',
    fontWeight: 500,
  } as React.CSSProperties,
  headerPath: {
    color: 'var(--text-muted)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
    flex: 1,
  } as React.CSSProperties,
  content: {
    flex: 1,
    overflow: 'auto',
    fontSize: 13,
    lineHeight: '20px',
  } as React.CSSProperties,
  line: {
    display: 'flex',
    minHeight: 20,
    transition: 'background 0.05s',
  } as React.CSSProperties,
  lineHover: {
    background: 'var(--bg-hover)',
  } as React.CSSProperties,
  blameGutter: {
    width: 280,
    minWidth: 280,
    padding: '0 8px',
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    borderRight: '1px solid var(--bg-tertiary)',
    flexShrink: 0,
    overflow: 'hidden',
    cursor: 'pointer',
  } as React.CSSProperties,
  blameGutterSameCommit: {
    color: 'var(--border-color)',
  } as React.CSSProperties,
  blameHash: {
    fontSize: 11,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    color: '#569cd6',
    flexShrink: 0,
    width: 60,
  } as React.CSSProperties,
  blameAuthor: {
    fontSize: 11,
    color: 'var(--text-secondary)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
    width: 80,
    flexShrink: 0,
  } as React.CSSProperties,
  blameDate: {
    fontSize: 10,
    color: 'var(--text-muted)',
    flexShrink: 0,
    marginLeft: 'auto',
    whiteSpace: 'nowrap' as const,
  } as React.CSSProperties,
  lineNumber: {
    width: 48,
    minWidth: 48,
    textAlign: 'right' as const,
    padding: '0 8px',
    fontSize: 12,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    color: 'var(--text-muted)',
    userSelect: 'none' as const,
    flexShrink: 0,
  } as React.CSSProperties,
  lineContent: {
    flex: 1,
    padding: '0 12px',
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    fontSize: 13,
    color: '#d4d4d4',
    whiteSpace: 'pre' as const,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  } as React.CSSProperties,
  loading: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 40,
    color: 'var(--text-secondary)',
    fontSize: 13,
  } as React.CSSProperties,
  error: {
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 40,
    color: '#f44747',
    fontSize: 13,
    gap: 8,
  } as React.CSSProperties,
  tooltip: {
    position: 'absolute' as const,
    zIndex: 100,
    background: 'var(--bg-secondary)',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
    padding: '8px 12px',
    fontSize: 12,
    color: 'var(--text-primary)',
    maxWidth: 400,
    lineHeight: 1.5,
  } as React.CSSProperties,
  tooltipHash: {
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    color: '#569cd6',
    fontSize: 11,
    marginBottom: 4,
  } as React.CSSProperties,
  tooltipMessage: {
    color: 'var(--text-primary)',
    marginBottom: 4,
    fontWeight: 500,
  } as React.CSSProperties,
  tooltipMeta: {
    color: 'var(--text-muted)',
    fontSize: 11,
  } as React.CSSProperties,
};

interface BlameLineProps {
  annotation: BlameAnnotation;
  showGutter: boolean;
  onHover: (hash: string | null) => void;
  hoveredHash: string | null;
}

function BlameLine({ annotation, showGutter, onHover, hoveredHash }: BlameLineProps) {
  const isHighlighted = hoveredHash === annotation.hash;

  return (
    <div
      style={{
        ...styles.line,
        background: isHighlighted ? 'var(--bg-hover)' : 'transparent',
      }}
      onMouseEnter={() => {
        onHover(annotation.hash);
      }}
      onMouseLeave={() => {
        onHover(null);
      }}
    >
      <div
        style={{
          ...styles.blameGutter,
          ...(showGutter ? {} : styles.blameGutterSameCommit),
        }}
        title={`${annotation.hash}\n${annotation.message}\n${annotation.author} - ${annotation.date}`}
      >
        {showGutter ? (
          <>
            <span style={styles.blameHash}>{annotation.shortHash}</span>
            <span style={styles.blameAuthor}>{annotation.author}</span>
            <span style={styles.blameDate}>{annotation.relativeDate}</span>
          </>
        ) : (
          <span style={{ color: 'var(--bg-tertiary)' }}>|</span>
        )}
      </div>
      <span style={styles.lineNumber}>{annotation.lineNumber}</span>
      <span style={styles.lineContent}>{annotation.lineContent}</span>
    </div>
  );
}

export default function BlameView({ filePath, projectPath }: BlameViewProps) {
  const [annotations, setAnnotations] = useState<BlameAnnotation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [hoveredHash, setHoveredHash] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    setError('');

    invoke<BlameAnnotation[]>('git_blame', {
      path: projectPath,
      filePath,
    })
      .then((result) => {
        if (!cancelled) {
          setAnnotations(result);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(`Failed to load blame: ${err}`);
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [filePath, projectPath]);

  const fileName = filePath.split('/').pop() || filePath;

  if (loading) {
    return (
      <div style={styles.container}>
        <div style={styles.header}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="var(--text-secondary)">
            <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 13A6 6 0 118 2a6 6 0 010 12z" />
          </svg>
          <span style={styles.headerTitle}>Git Blame</span>
          <span style={styles.headerPath}>{filePath}</span>
        </div>
        <div style={styles.loading}>Loading blame annotations...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div style={styles.container}>
        <div style={styles.header}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="#f44747">
            <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 13A6 6 0 118 2a6 6 0 010 12z" />
          </svg>
          <span style={styles.headerTitle}>Git Blame</span>
          <span style={styles.headerPath}>{filePath}</span>
        </div>
        <div style={styles.error}>
          <span>{error}</span>
        </div>
      </div>
    );
  }

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <svg width="14" height="14" viewBox="0 0 16 16" fill="#569cd6">
          <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 13A6 6 0 118 2a6 6 0 010 12zm-.5-9.5v4h1v-4h-1zm0 5v1h1v-1h-1z" />
        </svg>
        <span style={styles.headerTitle}>Git Blame</span>
        <span style={styles.headerPath}>{fileName}</span>
        <span style={{ marginLeft: 'auto', fontSize: 11, color: 'var(--text-muted)' }}>
          {annotations.length} lines
        </span>
      </div>
      <div style={styles.content}>
        {annotations.map((annotation, index) => {
          // Show gutter only for first line of a commit block
          const prevAnnotation = index > 0 ? annotations[index - 1] : null;
          const showGutter = !prevAnnotation || prevAnnotation.hash !== annotation.hash;

          return (
            <BlameLine
              key={annotation.lineNumber}
              annotation={annotation}
              showGutter={showGutter}
              onHover={setHoveredHash}
              hoveredHash={hoveredHash}
            />
          );
        })}
      </div>
    </div>
  );
}
