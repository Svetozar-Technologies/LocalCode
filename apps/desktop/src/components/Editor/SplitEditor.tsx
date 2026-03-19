import { useState, useRef, useCallback, useEffect } from 'react';
import Editor, { DiffEditor } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';

interface SplitEditorProps {
  leftPath: string;
  rightPath: string;
  diffMode?: boolean;
}

type SplitDirection = 'horizontal' | 'vertical';

const styles = {
  container: {
    display: 'flex',
    width: '100%',
    height: '100%',
    overflow: 'hidden',
    background: 'var(--bg-primary)',
  } as React.CSSProperties,
  toolbar: {
    display: 'flex',
    alignItems: 'center',
    gap: 4,
    padding: '4px 8px',
    background: 'var(--bg-secondary)',
    borderBottom: '1px solid #3c3c3c',
    fontSize: 12,
    color: 'var(--text-secondary)',
    flexShrink: 0,
  } as React.CSSProperties,
  toolbarButton: {
    background: 'none',
    border: '1px solid #3c3c3c',
    borderRadius: 3,
    color: 'var(--text-primary)',
    padding: '2px 8px',
    cursor: 'pointer',
    fontSize: 11,
  } as React.CSSProperties,
  toolbarButtonActive: {
    background: '#007acc',
    borderColor: '#007acc',
    color: '#ffffff',
  } as React.CSSProperties,
  pane: {
    flex: 1,
    overflow: 'hidden',
    display: 'flex',
    flexDirection: 'column' as const,
    minWidth: 0,
    minHeight: 0,
  } as React.CSSProperties,
  paneHeader: {
    padding: '4px 12px',
    background: 'var(--bg-tertiary)',
    borderBottom: '1px solid #3c3c3c',
    fontSize: 12,
    color: 'var(--text-primary)',
    whiteSpace: 'nowrap' as const,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    flexShrink: 0,
  } as React.CSSProperties,
  divider: {
    background: 'var(--border-color)',
    flexShrink: 0,
    transition: 'background 0.15s',
  } as React.CSSProperties,
  editorWrapper: {
    flex: 1,
    overflow: 'hidden',
  } as React.CSSProperties,
};

const editorOptions: editor.IStandaloneEditorConstructionOptions = {
  fontSize: 14,
  fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
  fontLigatures: true,
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
  renderWhitespace: 'selection',
  bracketPairColorization: { enabled: true },
  guides: { bracketPairs: true, indentation: true },
  smoothScrolling: true,
  cursorBlinking: 'smooth',
  cursorSmoothCaretAnimation: 'on',
  padding: { top: 8 },
  tabSize: 2,
  automaticLayout: true,
  readOnly: false,
};

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact',
    py: 'python', rs: 'rust', go: 'go', java: 'java', c: 'c', cpp: 'cpp', h: 'c',
    html: 'html', css: 'css', scss: 'scss', json: 'json', md: 'markdown',
    yml: 'yaml', yaml: 'yaml', toml: 'toml', sh: 'shell', sql: 'sql',
    xml: 'xml', svg: 'xml', graphql: 'graphql', swift: 'swift', kt: 'kotlin',
    rb: 'ruby', php: 'php', lua: 'lua', zig: 'zig', svelte: 'svelte', vue: 'vue',
  };
  return map[ext] || 'plaintext';
}

export default function SplitEditor({ leftPath, rightPath, diffMode = false }: SplitEditorProps) {
  const [direction, setDirection] = useState<SplitDirection>('horizontal');
  const [leftContent, setLeftContent] = useState('');
  const [rightContent, setRightContent] = useState('');
  const [loading, setLoading] = useState(true);
  const [splitRatio, setSplitRatio] = useState(0.5);
  const containerRef = useRef<HTMLDivElement>(null);
  const draggingRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    Promise.all([
      invoke<string>('read_file', { path: leftPath }),
      invoke<string>('read_file', { path: rightPath }),
    ])
      .then(([left, right]) => {
        if (!cancelled) {
          setLeftContent(left);
          setRightContent(right);
          setLoading(false);
        }
      })
      .catch((err) => {
        console.error('Failed to load files for split editor:', err);
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [leftPath, rightPath]);

  const handleDividerMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      draggingRef.current = true;
      const container = containerRef.current;
      if (!container) return;

      const rect = container.getBoundingClientRect();
      const isHorizontal = direction === 'horizontal';

      const handleMouseMove = (ev: MouseEvent) => {
        if (!draggingRef.current) return;
        const pos = isHorizontal ? ev.clientX - rect.left : ev.clientY - rect.top;
        const total = isHorizontal ? rect.width : rect.height;
        const ratio = Math.max(0.15, Math.min(0.85, pos / total));
        setSplitRatio(ratio);
      };

      const handleMouseUp = () => {
        draggingRef.current = false;
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = isHorizontal ? 'col-resize' : 'row-resize';
      document.body.style.userSelect = 'none';
    },
    [direction]
  );

  const leftName = leftPath.split('/').pop() || leftPath;
  const rightName = rightPath.split('/').pop() || rightPath;
  const leftLang = getLanguageFromPath(leftPath);
  const rightLang = getLanguageFromPath(rightPath);

  if (loading) {
    return (
      <div style={{ ...styles.container, alignItems: 'center', justifyContent: 'center', color: 'var(--text-secondary)' }}>
        Loading files...
      </div>
    );
  }

  if (diffMode) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', width: '100%', height: '100%' }}>
        <div style={styles.toolbar}>
          <span>Diff: {leftName} vs {rightName}</span>
        </div>
        <div style={{ flex: 1, overflow: 'hidden' }}>
          <DiffEditor
            height="100%"
            original={leftContent}
            modified={rightContent}
            originalLanguage={leftLang}
            modifiedLanguage={rightLang}
            theme="vs-dark"
            options={{
              ...editorOptions,
              readOnly: true,
              renderSideBySide: true,
            }}
          />
        </div>
      </div>
    );
  }

  const isHorizontal = direction === 'horizontal';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', width: '100%', height: '100%' }}>
      <div style={styles.toolbar}>
        <span style={{ marginRight: 8 }}>Split Editor</span>
        <button
          style={{
            ...styles.toolbarButton,
            ...(direction === 'horizontal' ? styles.toolbarButtonActive : {}),
          }}
          onClick={() => setDirection('horizontal')}
          title="Split Horizontal"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M14 1H2L1 2v12l1 1h12l1-1V2l-1-1zM7 14H2V2h5v12zm7 0H8V2h6v12z" />
          </svg>
        </button>
        <button
          style={{
            ...styles.toolbarButton,
            ...(direction === 'vertical' ? styles.toolbarButtonActive : {}),
          }}
          onClick={() => setDirection('vertical')}
          title="Split Vertical"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M14 1H2L1 2v12l1 1h12l1-1V2l-1-1zM14 7H2V2h12v5zM14 14H2V8h12v6z" />
          </svg>
        </button>
      </div>
      <div
        ref={containerRef}
        style={{
          ...styles.container,
          flexDirection: isHorizontal ? 'row' : 'column',
          flex: 1,
        }}
      >
        <div
          style={{
            ...styles.pane,
            [isHorizontal ? 'width' : 'height']: `${splitRatio * 100}%`,
            [isHorizontal ? 'height' : 'width']: '100%',
            flex: 'none',
          }}
        >
          <div style={styles.paneHeader} title={leftPath}>
            {leftName}
          </div>
          <div style={styles.editorWrapper}>
            <Editor
              height="100%"
              language={leftLang}
              value={leftContent}
              theme="vs-dark"
              onChange={(value) => {
                if (value !== undefined) setLeftContent(value);
              }}
              options={editorOptions}
            />
          </div>
        </div>

        <div
          style={{
            ...styles.divider,
            [isHorizontal ? 'width' : 'height']: 4,
            cursor: isHorizontal ? 'col-resize' : 'row-resize',
          }}
          onMouseDown={handleDividerMouseDown}
          onMouseEnter={(e) => {
            (e.target as HTMLElement).style.background = '#007acc';
          }}
          onMouseLeave={(e) => {
            if (!draggingRef.current) {
              (e.target as HTMLElement).style.background = 'var(--border-color)';
            }
          }}
        />

        <div style={styles.pane}>
          <div style={styles.paneHeader} title={rightPath}>
            {rightName}
          </div>
          <div style={styles.editorWrapper}>
            <Editor
              height="100%"
              language={rightLang}
              value={rightContent}
              theme="vs-dark"
              onChange={(value) => {
                if (value !== undefined) setRightContent(value);
              }}
              options={editorOptions}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
