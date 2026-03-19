import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';

interface QuickOpenProps {
  visible: boolean;
  onClose: () => void;
}

interface FileResult {
  path: string;
  name: string;
  relativePath: string;
}

const styles = {
  backdrop: {
    position: 'fixed' as const,
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    zIndex: 1000,
    display: 'flex',
    justifyContent: 'center',
    paddingTop: 80,
  } as React.CSSProperties,
  container: {
    width: 560,
    maxHeight: 420,
    background: 'var(--bg-secondary)',
    border: '1px solid var(--border-color)',
    borderRadius: 6,
    boxShadow: '0 12px 40px rgba(0, 0, 0, 0.5)',
    display: 'flex',
    flexDirection: 'column' as const,
    overflow: 'hidden',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  inputWrapper: {
    display: 'flex',
    alignItems: 'center',
    padding: '8px 12px',
    borderBottom: '1px solid var(--border-color)',
    gap: 8,
  } as React.CSSProperties,
  searchIcon: {
    color: 'var(--text-secondary)',
    flexShrink: 0,
  } as React.CSSProperties,
  input: {
    flex: 1,
    background: 'transparent',
    border: 'none',
    color: 'var(--text-primary)',
    fontSize: 14,
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    outline: 'none',
    padding: '4px 0',
  } as React.CSSProperties,
  results: {
    flex: 1,
    overflowY: 'auto' as const,
    maxHeight: 360,
  } as React.CSSProperties,
  resultItem: {
    display: 'flex',
    alignItems: 'center',
    padding: '6px 12px',
    cursor: 'pointer',
    gap: 10,
    fontSize: 13,
    transition: 'background 0.05s',
  } as React.CSSProperties,
  resultItemActive: {
    background: '#062f4a',
  } as React.CSSProperties,
  resultItemHover: {
    background: 'var(--bg-hover)',
  } as React.CSSProperties,
  fileIcon: {
    flexShrink: 0,
    color: 'var(--text-primary)',
  } as React.CSSProperties,
  fileName: {
    color: 'var(--text-primary)',
    fontWeight: 500,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  } as React.CSSProperties,
  filePath: {
    color: 'var(--text-muted)',
    fontSize: 11,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
    marginLeft: 'auto',
    flexShrink: 0,
    maxWidth: '50%',
  } as React.CSSProperties,
  highlight: {
    color: '#007acc',
    fontWeight: 600,
  } as React.CSSProperties,
  empty: {
    padding: 20,
    textAlign: 'center' as const,
    color: 'var(--text-muted)',
    fontSize: 13,
  } as React.CSSProperties,
  loading: {
    padding: 16,
    textAlign: 'center' as const,
    color: 'var(--text-secondary)',
    fontSize: 12,
  } as React.CSSProperties,
  footer: {
    display: 'flex',
    alignItems: 'center',
    gap: 12,
    padding: '6px 12px',
    borderTop: '1px solid var(--border-color)',
    fontSize: 11,
    color: 'var(--text-muted)',
  } as React.CSSProperties,
  footerKbd: {
    background: '#2d2d2d',
    border: '1px solid var(--border-color)',
    borderRadius: 3,
    padding: '1px 5px',
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    fontSize: 10,
    color: 'var(--text-secondary)',
  } as React.CSSProperties,
};

const EXT_COLORS: Record<string, string> = {
  ts: '#3178c6', tsx: '#3178c6', js: '#f7df1e', jsx: '#f7df1e',
  py: '#3776ab', rs: '#dea584', go: '#00add8', java: '#b07219',
  html: '#e34c26', css: '#1572b6', json: '#292929', md: '#083fa1',
};

function getFileColor(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase() || '';
  return EXT_COLORS[ext] || 'var(--text-primary)';
}

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact',
    py: 'python', rs: 'rust', go: 'go', java: 'java', c: 'c', cpp: 'cpp', h: 'c',
    html: 'html', css: 'css', scss: 'scss', json: 'json', md: 'markdown',
    yml: 'yaml', yaml: 'yaml', toml: 'toml', sh: 'shell', sql: 'sql',
  };
  return map[ext] || 'plaintext';
}

function highlightMatch(text: string, query: string): React.ReactNode[] {
  if (!query) return [text];

  const parts: React.ReactNode[] = [];
  let lastIndex = 0;
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();

  // Simple fuzzy matching: highlight characters in order
  let qi = 0;
  for (let i = 0; i < text.length && qi < lowerQuery.length; i++) {
    if (lowerText[i] === lowerQuery[qi]) {
      if (i > lastIndex) {
        parts.push(text.slice(lastIndex, i));
      }
      parts.push(
        <span key={`h-${i}`} style={styles.highlight}>{text[i]}</span>
      );
      lastIndex = i + 1;
      qi++;
    }
  }
  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex));
  }

  return parts;
}

export default function QuickOpen({ visible, onClose }: QuickOpenProps) {
  const { projectPath, openFile } = useAppStore();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<FileResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const resultRefs = useRef<(HTMLDivElement | null)[]>([]);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Focus input when visible
  useEffect(() => {
    if (visible) {
      setQuery('');
      setResults([]);
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [visible]);

  // Search files with debounce
  useEffect(() => {
    if (!visible || !projectPath) return;

    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    if (!query.trim()) {
      setResults([]);
      setLoading(false);
      return;
    }

    setLoading(true);

    debounceRef.current = setTimeout(async () => {
      try {
        const files = await invoke<string[]>('search_files', {
          path: projectPath,
          pattern: query,
        });

        const fileResults: FileResult[] = files.slice(0, 50).map((filePath) => {
          const name = filePath.split('/').pop() || filePath;
          const relativePath = filePath.startsWith(projectPath)
            ? filePath.slice(projectPath.length + 1)
            : filePath;
          return { path: filePath, name, relativePath };
        });

        setResults(fileResults);
        setSelectedIndex(0);
      } catch (err) {
        console.error('Quick open search failed:', err);
        setResults([]);
      }
      setLoading(false);
    }, 150);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [query, visible, projectPath]);

  // Scroll selected item into view
  useEffect(() => {
    resultRefs.current[selectedIndex]?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  const handleOpen = useCallback(
    async (file: FileResult) => {
      try {
        const content = await invoke<string>('read_file', { path: file.path });
        openFile({
          path: file.path,
          name: file.name,
          content,
          language: getLanguageFromPath(file.path),
          modified: false,
        });
        onClose();
      } catch (err) {
        console.error('Failed to open file:', err);
      }
    },
    [openFile, onClose]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case 'Escape':
          e.preventDefault();
          onClose();
          break;
        case 'ArrowDown':
          e.preventDefault();
          setSelectedIndex((prev) => Math.min(prev + 1, results.length - 1));
          break;
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
          break;
        case 'Enter':
          e.preventDefault();
          if (results[selectedIndex]) {
            handleOpen(results[selectedIndex]);
          }
          break;
      }
    },
    [results, selectedIndex, handleOpen, onClose]
  );

  if (!visible) return null;

  return (
    <div
      style={styles.backdrop}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div style={styles.container}>
        <div style={styles.inputWrapper}>
          <svg style={styles.searchIcon} width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M15.25 0a8.25 8.25 0 0 0-6.18 13.72L1 21.75l1.27 1.27 8.05-8.04A8.25 8.25 0 1 0 15.25 0zm0 14.5a6.25 6.25 0 1 1 0-12.5 6.25 6.25 0 0 1 0 12.5z" />
          </svg>
          <input
            ref={inputRef}
            style={styles.input}
            type="text"
            placeholder="Search files by name..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
          />
        </div>

        <div style={styles.results}>
          {loading && (
            <div style={styles.loading}>Searching...</div>
          )}
          {!loading && query && results.length === 0 && (
            <div style={styles.empty}>No files found matching "{query}"</div>
          )}
          {!loading && !query && (
            <div style={styles.empty}>Start typing to search files</div>
          )}
          {!loading &&
            results.map((file, index) => (
              <div
                key={file.path}
                ref={(el) => { resultRefs.current[index] = el; }}
                style={{
                  ...styles.resultItem,
                  ...(index === selectedIndex ? styles.resultItemActive : {}),
                }}
                onClick={() => handleOpen(file)}
                onMouseEnter={() => setSelectedIndex(index)}
              >
                <svg style={{ ...styles.fileIcon, color: getFileColor(file.name) }} width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M13.71 4.29l-3-3L10 1H4L3 2v12l1 1h9l1-1V5l-.29-.71zM13 14H4V2h5v4h4v8z" />
                </svg>
                <span style={styles.fileName}>
                  {highlightMatch(file.name, query)}
                </span>
                <span style={styles.filePath} title={file.relativePath}>
                  {file.relativePath}
                </span>
              </div>
            ))}
        </div>

        <div style={styles.footer}>
          <span>
            <kbd style={styles.footerKbd}>↑↓</kbd> navigate
          </span>
          <span>
            <kbd style={styles.footerKbd}>Enter</kbd> open
          </span>
          <span>
            <kbd style={styles.footerKbd}>Esc</kbd> close
          </span>
          {results.length > 0 && (
            <span style={{ marginLeft: 'auto' }}>
              {results.length} file{results.length !== 1 ? 's' : ''}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
