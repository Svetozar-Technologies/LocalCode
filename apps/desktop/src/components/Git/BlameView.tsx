import { useState, useEffect, useCallback } from 'react';
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
    background: '#1e1e1e',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    overflow: 'auto',
  } as React.CSSProperties,
  header: {
    display: 'flex',
    alignItems: 'center',
    padding: '8px 12px',
    background: '#252526',
    borderBottom: '1px solid #3c3c3c',
    fontSize: 12,
    color: '#969696',
    gap: 8,
    flexShrink: 0,
  } as React.CSSProperties,
  headerTitle: {
    color: '#cccccc',
    fontWeight: 500,
  } as React.CSSProperties,
  headerPath: {
    color: '#6a6a6a',
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
    background: '#2a2d2e',
  } as React.CSSProperties,
  blameGutter: {
    width: 280,
    minWidth: 280,
    padding: '0 8px',
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    borderRight: '1px solid #2d2d2d',
    flexShrink: 0,
    overflow: 'hidden',
    cursor: 'pointer',
  } as React.CSSProperties,
  blameGutterSameCommit: {
    color: '#3c3c3c',
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
    color: '#969696',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
    width: 80,
    flexShrink: 0,
  } as React.CSSProperties,
  blameDate: {
    fontSize: 10,
    color: '#6a6a6a',
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
    color: '#6a6a6a',
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
    color: '#969696',
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
    background: '#252526',
    border: '1px solid #3c3c3c',
    borderRadius: 4,
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
    padding: '8px 12px',
    fontSize: 12,
    color: '#cccccc',
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
    color: '#cccccc',
    marginBottom: 4,
    fontWeight: 500,
  } as React.CSSProperties,
  tooltipMeta: {
    color: '#6a6a6a',
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
  const [hovered, setHovered] = useState(false);
  const isHighlighted = hoveredHash === annotation.hash;

  return (
    <div
      style={{
        ...styles.line,
        background: isHighlighted ? '#2a2d2e' : 'transparent',
      }}
      onMouseEnter={() => {
        setHovered(true);
        onHover(annotation.hash);
      }}
      onMouseLeave={() => {
        setHovered(false);
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
          <span style={{ color: '#2d2d2d' }}>|</span>
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

  const handleCopyHash = useCallback((hash: string) => {
    navigator.clipboard.writeText(hash).catch(() => {});
  }, []);

  const fileName = filePath.split('/').pop() || filePath;

  if (loading) {
    return (
      <div style={styles.container}>
        <div style={styles.header}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="#969696">
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
        <span style={{ marginLeft: 'auto', fontSize: 11, color: '#6a6a6a' }}>
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
