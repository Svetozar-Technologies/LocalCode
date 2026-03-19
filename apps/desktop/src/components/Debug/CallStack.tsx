interface StackFrame {
  id: number;
  name: string;
  file?: string;
  line: number;
  column: number;
}

interface CallStackProps {
  frames: StackFrame[];
  onFrameClick?: (frame: StackFrame) => void;
}

const styles = {
  container: {
    fontSize: 12,
  } as React.CSSProperties,
  frame: {
    display: 'flex',
    alignItems: 'center',
    padding: '3px 0',
    gap: 8,
    cursor: 'pointer',
    borderRadius: 3,
  } as React.CSSProperties,
  frameHover: {
    background: 'var(--bg-hover)',
  } as React.CSSProperties,
  frameIndex: {
    color: 'var(--text-muted)',
    fontSize: 10,
    width: 16,
    textAlign: 'right' as const,
    flexShrink: 0,
  } as React.CSSProperties,
  frameName: {
    color: '#dcdcaa',
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    fontSize: 12,
  } as React.CSSProperties,
  frameLocation: {
    color: 'var(--text-muted)',
    fontSize: 11,
    marginLeft: 'auto',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
    maxWidth: 200,
    flexShrink: 0,
  } as React.CSSProperties,
  empty: {
    color: 'var(--text-muted)',
    fontSize: 12,
    padding: '8px 0',
  } as React.CSSProperties,
};

export default function CallStack({ frames, onFrameClick }: CallStackProps) {
  if (frames.length === 0) {
    return <div style={styles.empty}>No call stack available</div>;
  }

  return (
    <div style={styles.container}>
      {frames.map((frame, index) => {
        const fileName = frame.file?.split('/').pop() || 'unknown';
        return (
          <div
            key={frame.id}
            style={styles.frame}
            onClick={() => onFrameClick?.(frame)}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = 'var(--bg-hover)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = 'transparent';
            }}
            title={`${frame.name} at ${frame.file || 'unknown'}:${frame.line}`}
          >
            <span style={styles.frameIndex}>{index}</span>
            <span style={styles.frameName}>{frame.name}</span>
            <span style={styles.frameLocation}>
              {fileName}:{frame.line}
            </span>
          </div>
        );
      })}
    </div>
  );
}
