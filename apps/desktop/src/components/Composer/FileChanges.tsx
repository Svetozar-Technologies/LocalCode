import { useState, useCallback } from 'react';
import { DiffEditor } from '@monaco-editor/react';
import type { FileChange } from './Composer';

interface FileChangesProps {
  changes: FileChange[];
  onAccept: (path: string) => void;
  onReject: (path: string) => void;
}

const styles = {
  fileItem: {
    borderBottom: '1px solid #3c3c3c',
  } as React.CSSProperties,
  fileHeader: {
    display: 'flex',
    alignItems: 'center',
    padding: '8px 16px',
    cursor: 'pointer',
    gap: 8,
    fontSize: 13,
    transition: 'background 0.1s',
    background: '#252526',
  } as React.CSSProperties,
  fileHeaderHover: {
    background: '#2a2d2e',
  } as React.CSSProperties,
  chevron: {
    width: 14,
    height: 14,
    color: '#969696',
    flexShrink: 0,
    transition: 'transform 0.15s',
  } as React.CSSProperties,
  chevronExpanded: {
    transform: 'rotate(90deg)',
  } as React.CSSProperties,
  fileIcon: {
    flexShrink: 0,
  } as React.CSSProperties,
  fileName: {
    color: '#cccccc',
    fontWeight: 500,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  } as React.CSSProperties,
  filePath: {
    color: '#6a6a6a',
    fontSize: 11,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
    flex: 1,
    minWidth: 0,
  } as React.CSSProperties,
  statusBadge: {
    fontSize: 10,
    padding: '2px 8px',
    borderRadius: 8,
    fontWeight: 600,
    flexShrink: 0,
  } as React.CSSProperties,
  actions: {
    display: 'flex',
    gap: 4,
    marginLeft: 'auto',
    flexShrink: 0,
  } as React.CSSProperties,
  actionButton: {
    width: 26,
    height: 26,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'none',
    border: '1px solid transparent',
    borderRadius: 4,
    cursor: 'pointer',
    flexShrink: 0,
    transition: 'all 0.1s',
  } as React.CSSProperties,
  acceptButton: {
    color: '#4ec9b0',
    borderColor: '#4ec9b044',
  } as React.CSSProperties,
  rejectButton: {
    color: '#f44747',
    borderColor: '#f4474744',
  } as React.CSSProperties,
  diffArea: {
    height: 300,
    borderTop: '1px solid #3c3c3c',
  } as React.CSSProperties,
  statsRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '4px 16px',
    background: '#1e1e1e',
    borderTop: '1px solid #3c3c3c',
    fontSize: 11,
    color: '#6a6a6a',
  } as React.CSSProperties,
  emptyState: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 40,
    color: '#6a6a6a',
    fontSize: 13,
  } as React.CSSProperties,
};

const EXT_COLORS: Record<string, string> = {
  ts: '#3178c6', tsx: '#3178c6', js: '#f7df1e', jsx: '#f7df1e',
  py: '#3776ab', rs: '#dea584', go: '#00add8', java: '#b07219',
  html: '#e34c26', css: '#1572b6', json: '#292929', md: '#083fa1',
};

function getFileColor(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase() || '';
  return EXT_COLORS[ext] || '#cccccc';
}

function computeStats(original: string, proposed: string): { added: number; removed: number } {
  const origLines = original.split('\n');
  const propLines = proposed.split('\n');
  const origSet = new Set(origLines);
  const propSet = new Set(propLines);

  let added = 0;
  let removed = 0;
  for (const line of propLines) {
    if (!origSet.has(line)) added++;
  }
  for (const line of origLines) {
    if (!propSet.has(line)) removed++;
  }
  return { added, removed };
}

function getStatusStyle(status: FileChange['status']): React.CSSProperties {
  switch (status) {
    case 'accepted':
      return { background: '#4ec9b022', color: '#4ec9b0', border: '1px solid #4ec9b044' };
    case 'rejected':
      return { background: '#f4474722', color: '#f44747', border: '1px solid #f4474744' };
    case 'pending':
    default:
      return { background: '#dcdcaa22', color: '#dcdcaa', border: '1px solid #dcdcaa44' };
  }
}

function FileChangeItem({
  change,
  onAccept,
  onReject,
}: {
  change: FileChange;
  onAccept: () => void;
  onReject: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [hovered, setHovered] = useState(false);

  const fileName = change.path.split('/').pop() || change.path;
  const dirPath = change.path.split('/').slice(0, -1).join('/');
  const stats = computeStats(change.originalContent, change.proposedContent);
  const isNew = !change.originalContent;

  return (
    <div style={styles.fileItem}>
      <div
        style={{
          ...styles.fileHeader,
          ...(hovered ? styles.fileHeaderHover : {}),
        }}
        onClick={() => setExpanded(!expanded)}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        <svg
          style={{
            ...styles.chevron,
            ...(expanded ? styles.chevronExpanded : {}),
          }}
          viewBox="0 0 16 16"
          fill="currentColor"
        >
          <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" fill="none" />
        </svg>

        <svg style={{ ...styles.fileIcon, color: getFileColor(fileName) }} width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M13.71 4.29l-3-3L10 1H4L3 2v12l1 1h9l1-1V5l-.29-.71zM13 14H4V2h5v4h4v8z" />
        </svg>

        <span style={styles.fileName}>{fileName}</span>

        {dirPath && <span style={styles.filePath}>{dirPath}</span>}

        <span style={{ fontSize: 11, color: '#4ec9b0', flexShrink: 0 }}>+{stats.added}</span>
        <span style={{ fontSize: 11, color: '#f44747', flexShrink: 0 }}>-{stats.removed}</span>

        {isNew && (
          <span style={{ ...styles.statusBadge, background: '#4ec9b022', color: '#4ec9b0' }}>
            NEW
          </span>
        )}

        <span style={{ ...styles.statusBadge, ...getStatusStyle(change.status) }}>
          {change.status}
        </span>

        {change.status === 'pending' && (
          <div style={styles.actions}>
            <button
              style={{ ...styles.actionButton, ...styles.acceptButton }}
              onClick={(e) => {
                e.stopPropagation();
                onAccept();
              }}
              title="Accept change"
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#4ec9b022'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                <path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z" />
              </svg>
            </button>
            <button
              style={{ ...styles.actionButton, ...styles.rejectButton }}
              onClick={(e) => {
                e.stopPropagation();
                onReject();
              }}
              title="Reject change"
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#f4474722'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                <path d="M8 8.707l3.646 3.647.708-.707L8.707 8l3.647-3.646-.707-.708L8 7.293 4.354 3.646l-.707.708L7.293 8l-3.646 3.646.707.708L8 8.707z" />
              </svg>
            </button>
          </div>
        )}
      </div>

      {expanded && (
        <div style={styles.diffArea}>
          <DiffEditor
            height="100%"
            original={change.originalContent}
            modified={change.proposedContent}
            language={change.language}
            theme="vs-dark"
            options={{
              fontSize: 13,
              fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              readOnly: true,
              originalEditable: false,
              renderSideBySide: true,
              automaticLayout: true,
              padding: { top: 4 },
            }}
          />
        </div>
      )}
    </div>
  );
}

export default function FileChanges({ changes, onAccept, onReject }: FileChangesProps) {
  if (changes.length === 0) {
    return <div style={styles.emptyState}>No file changes yet.</div>;
  }

  return (
    <div>
      {changes.map((change) => (
        <FileChangeItem
          key={change.path}
          change={change}
          onAccept={() => onAccept(change.path)}
          onReject={() => onReject(change.path)}
        />
      ))}
    </div>
  );
}
