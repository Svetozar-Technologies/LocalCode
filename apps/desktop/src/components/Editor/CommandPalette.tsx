import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import type { editor } from 'monaco-editor';
import { useAppStore } from '../../stores/appStore';

interface MonacoWindow extends Window {
  monaco?: {
    editor: {
      getEditors?: () => editor.IStandaloneCodeEditor[];
      EditorOption: typeof editor.EditorOption;
    };
  };
}

interface Command {
  id: string;
  label: string;
  shortcut?: string;
  category: string;
  action: () => void;
}

interface CommandPaletteProps {
  visible: boolean;
  onClose: () => void;
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
  label: {
    color: 'var(--text-primary)',
    flex: 1,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  } as React.CSSProperties,
  category: {
    color: 'var(--text-muted)',
    fontSize: 11,
    flexShrink: 0,
  } as React.CSSProperties,
  shortcut: {
    color: 'var(--text-secondary)',
    fontSize: 11,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    flexShrink: 0,
    background: '#2d2d2d',
    border: '1px solid var(--border-color)',
    borderRadius: 3,
    padding: '1px 5px',
  } as React.CSSProperties,
  highlight: {
    color: '#007acc',
    fontWeight: 600,
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

function fuzzyMatch(text: string, query: string): boolean {
  if (!query) return true;
  const lower = text.toLowerCase();
  const q = query.toLowerCase();
  let qi = 0;
  for (let i = 0; i < lower.length && qi < q.length; i++) {
    if (lower[i] === q[qi]) qi++;
  }
  return qi === q.length;
}

function highlightMatch(text: string, query: string): React.ReactNode[] {
  if (!query) return [text];
  const parts: React.ReactNode[] = [];
  let lastIndex = 0;
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  let qi = 0;
  for (let i = 0; i < text.length && qi < lowerQuery.length; i++) {
    if (lowerText[i] === lowerQuery[qi]) {
      if (i > lastIndex) parts.push(text.slice(lastIndex, i));
      parts.push(<span key={`h-${i}`} style={styles.highlight}>{text[i]}</span>);
      lastIndex = i + 1;
      qi++;
    }
  }
  if (lastIndex < text.length) parts.push(text.slice(lastIndex));
  return parts;
}

export default function CommandPalette({ visible, onClose }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const resultRefs = useRef<(HTMLDivElement | null)[]>([]);

  const commands: Command[] = useMemo(() => {
    const store = useAppStore.getState;
    const themes = ['dark', 'light', 'monokai', 'solarized'];
    return [
      {
        id: 'toggle-terminal',
        label: 'Toggle Terminal',
        shortcut: 'Cmd+`',
        category: 'View',
        action: () => store().toggleTerminal(),
      },
      {
        id: 'toggle-sidebar',
        label: 'Toggle Sidebar',
        shortcut: 'Cmd+B',
        category: 'View',
        action: () => {
          const s = store();
          s.setSidebarWidth(s.sidebarWidth > 0 ? 0 : 260);
        },
      },
      {
        id: 'toggle-chat',
        label: 'Toggle AI Chat',
        shortcut: 'Cmd+I',
        category: 'AI',
        action: () => store().toggleChatPanel(),
      },
      {
        id: 'toggle-agent',
        label: 'Toggle Agent Mode',
        category: 'AI',
        action: () => store().toggleAgentMode(),
      },
      {
        id: 'open-settings',
        label: 'Open Settings',
        category: 'View',
        action: () => {
          const s = store();
          s.setSidebarView('settings');
          if (s.sidebarWidth === 0) s.setSidebarWidth(260);
        },
      },
      {
        id: 'open-composer',
        label: 'Open Composer',
        shortcut: 'Cmd+Shift+I',
        category: 'AI',
        action: () => {
          const s = store();
          s.setSidebarView('composer');
          if (s.sidebarWidth === 0) s.setSidebarWidth(260);
        },
      },
      {
        id: 'toggle-theme',
        label: 'Toggle Theme',
        category: 'View',
        action: () => {
          const s = store();
          const idx = themes.indexOf(s.theme);
          s.setTheme(themes[(idx + 1) % themes.length]);
        },
      },
      {
        id: 'toggle-minimap',
        label: 'Toggle Minimap',
        category: 'Editor',
        action: () => {
          // Toggle via Monaco editor API
          const editors = (window as unknown as MonacoWindow).monaco?.editor?.getEditors?.() || [];
          for (const ed of editors) {
            const current = ed.getOption?.((window as unknown as MonacoWindow).monaco!.editor.EditorOption.minimap);
            ed.updateOptions({ minimap: { enabled: !current?.enabled } });
          }
        },
      },
      {
        id: 'toggle-wordwrap',
        label: 'Toggle Word Wrap',
        category: 'Editor',
        action: () => {
          const editors = (window as unknown as MonacoWindow).monaco?.editor?.getEditors?.() || [];
          for (const ed of editors) {
            const current = ed.getOption?.((window as unknown as MonacoWindow).monaco!.editor.EditorOption.wordWrap);
            ed.updateOptions({ wordWrap: current === 'on' ? 'off' : 'on' });
          }
        },
      },
      {
        id: 'format-document',
        label: 'Format Document',
        category: 'Editor',
        action: () => {
          const editors = (window as unknown as MonacoWindow).monaco?.editor?.getEditors?.() || [];
          if (editors[0]) {
            editors[0].getAction('editor.action.formatDocument')?.run();
          }
        },
      },
      {
        id: 'find-replace',
        label: 'Find and Replace',
        shortcut: 'Cmd+F',
        category: 'Editor',
        action: () => store().toggleFindReplace(),
      },
      {
        id: 'quick-open',
        label: 'Quick Open File',
        shortcut: 'Cmd+P',
        category: 'File',
        action: () => store().toggleQuickOpen(),
      },
      {
        id: 'git-commit',
        label: 'Git Commit',
        category: 'Git',
        action: () => {
          const s = store();
          s.setSidebarView('git');
          if (s.sidebarWidth === 0) s.setSidebarWidth(260);
        },
      },
      {
        id: 'show-problems',
        label: 'Show Problems',
        category: 'View',
        action: () => {
          const s = store();
          s.setBottomPanelTab('problems');
          if (!s.terminalVisible) s.toggleTerminal();
        },
      },
      {
        id: 'new-file',
        label: 'New File',
        category: 'File',
        action: () => {
          store().openFile({
            path: `untitled-${Date.now()}`,
            name: 'Untitled',
            content: '',
            language: 'plaintext',
            modified: true,
          });
        },
      },
      {
        id: 'inline-edit',
        label: 'Inline Edit',
        shortcut: 'Cmd+K',
        category: 'AI',
        action: () => store().setInlineEditVisible(true),
      },
      {
        id: 'show-explorer',
        label: 'Show Explorer',
        category: 'View',
        action: () => {
          const s = store();
          s.setSidebarView('explorer');
          if (s.sidebarWidth === 0) s.setSidebarWidth(260);
        },
      },
      {
        id: 'show-search',
        label: 'Show Search',
        shortcut: 'Cmd+Shift+F',
        category: 'View',
        action: () => {
          const s = store();
          s.setSidebarView('search');
          if (s.sidebarWidth === 0) s.setSidebarWidth(260);
        },
      },
      {
        id: 'show-debug',
        label: 'Show Debug Panel',
        category: 'View',
        action: () => {
          const s = store();
          s.setSidebarView('debug');
          if (s.sidebarWidth === 0) s.setSidebarWidth(260);
        },
      },
      {
        id: 'clear-chat',
        label: 'Clear Chat History',
        category: 'AI',
        action: () => store().clearChat(),
      },
    ];
  }, []);

  const filtered = useMemo(
    () => commands.filter((cmd) => fuzzyMatch(cmd.label, query) || fuzzyMatch(cmd.category, query)),
    [commands, query]
  );

  useEffect(() => {
    if (visible) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [visible]);

  useEffect(() => {
    resultRefs.current[selectedIndex]?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  const handleExecute = useCallback(
    (cmd: Command) => {
      onClose();
      // Run action after modal closes
      setTimeout(() => cmd.action(), 0);
    },
    [onClose]
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
          setSelectedIndex((prev) => Math.min(prev + 1, filtered.length - 1));
          break;
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
          break;
        case 'Enter':
          e.preventDefault();
          if (filtered[selectedIndex]) {
            handleExecute(filtered[selectedIndex]);
          }
          break;
      }
    },
    [filtered, selectedIndex, handleExecute, onClose]
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
          <span style={{ color: 'var(--text-secondary)', fontSize: 14 }}>&gt;</span>
          <input
            ref={inputRef}
            style={styles.input}
            type="text"
            placeholder="Type a command..."
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            onKeyDown={handleKeyDown}
          />
        </div>

        <div style={styles.results}>
          {filtered.length === 0 && query && (
            <div style={{ padding: 20, textAlign: 'center', color: 'var(--text-muted)', fontSize: 13 }}>
              No commands found
            </div>
          )}
          {filtered.map((cmd, index) => (
            <div
              key={cmd.id}
              ref={(el) => { resultRefs.current[index] = el; }}
              style={{
                ...styles.resultItem,
                ...(index === selectedIndex ? styles.resultItemActive : {}),
              }}
              onClick={() => handleExecute(cmd)}
              onMouseEnter={() => setSelectedIndex(index)}
            >
              <span style={styles.label}>
                {highlightMatch(cmd.label, query)}
              </span>
              <span style={styles.category}>{cmd.category}</span>
              {cmd.shortcut && (
                <span style={styles.shortcut}>{cmd.shortcut}</span>
              )}
            </div>
          ))}
        </div>

        <div style={styles.footer}>
          <span><kbd style={styles.footerKbd}>↑↓</kbd> navigate</span>
          <span><kbd style={styles.footerKbd}>Enter</kbd> run</span>
          <span><kbd style={styles.footerKbd}>Esc</kbd> close</span>
          {filtered.length > 0 && (
            <span style={{ marginLeft: 'auto' }}>
              {filtered.length} command{filtered.length !== 1 ? 's' : ''}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
