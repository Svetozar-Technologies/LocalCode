import { DiffEditor } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';

interface DiffViewerProps {
  original: string;
  modified: string;
  language?: string;
  originalLabel?: string;
  modifiedLabel?: string;
  renderSideBySide?: boolean;
}

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    width: '100%',
    height: '100%',
    background: '#1e1e1e',
  } as React.CSSProperties,
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '6px 12px',
    background: '#252526',
    borderBottom: '1px solid #3c3c3c',
    fontSize: 12,
    color: '#969696',
    flexShrink: 0,
  } as React.CSSProperties,
  labels: {
    display: 'flex',
    alignItems: 'center',
    gap: 16,
  } as React.CSSProperties,
  label: {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
  } as React.CSSProperties,
  deletedDot: {
    width: 8,
    height: 8,
    borderRadius: '50%',
    background: '#f44747',
  } as React.CSSProperties,
  addedDot: {
    width: 8,
    height: 8,
    borderRadius: '50%',
    background: '#4ec9b0',
  } as React.CSSProperties,
  editorWrapper: {
    flex: 1,
    overflow: 'hidden',
  } as React.CSSProperties,
  emptyState: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    color: '#6a6a6a',
    fontSize: 13,
  } as React.CSSProperties,
};

const diffEditorOptions: editor.IDiffEditorConstructionOptions = {
  fontSize: 14,
  fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
  fontLigatures: true,
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
  renderWhitespace: 'selection',
  smoothScrolling: true,
  padding: { top: 8 },
  tabSize: 2,
  automaticLayout: true,
  readOnly: true,
  originalEditable: false,
  renderSideBySide: true,
  enableSplitViewResizing: true,
  renderOverviewRuler: true,
  ignoreTrimWhitespace: false,
};

function computeDiffStats(original: string, modified: string): { added: number; removed: number } {
  const origLines = original.split('\n');
  const modLines = modified.split('\n');
  const origSet = new Set(origLines);
  const modSet = new Set(modLines);

  let added = 0;
  let removed = 0;

  for (const line of modLines) {
    if (!origSet.has(line)) added++;
  }
  for (const line of origLines) {
    if (!modSet.has(line)) removed++;
  }

  return { added, removed };
}

export default function DiffViewer({
  original,
  modified,
  language = 'plaintext',
  originalLabel = 'Original',
  modifiedLabel = 'Modified',
  renderSideBySide = true,
}: DiffViewerProps) {
  const stats = computeDiffStats(original, modified);
  const hasChanges = original !== modified;

  if (!hasChanges) {
    return (
      <div style={styles.container}>
        <div style={styles.header}>
          <span>No differences</span>
        </div>
        <div style={styles.emptyState}>
          The files are identical.
        </div>
      </div>
    );
  }

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <div style={styles.labels}>
          <span style={styles.label}>
            <span style={styles.deletedDot} />
            {originalLabel}
          </span>
          <span style={styles.label}>
            <span style={styles.addedDot} />
            {modifiedLabel}
          </span>
        </div>
        <div style={{ display: 'flex', gap: 12, fontSize: 11 }}>
          <span style={{ color: '#4ec9b0' }}>+{stats.added}</span>
          <span style={{ color: '#f44747' }}>-{stats.removed}</span>
        </div>
      </div>
      <div style={styles.editorWrapper}>
        <DiffEditor
          height="100%"
          original={original}
          modified={modified}
          language={language}
          theme="vs-dark"
          options={{
            ...diffEditorOptions,
            renderSideBySide,
          }}
        />
      </div>
    </div>
  );
}
