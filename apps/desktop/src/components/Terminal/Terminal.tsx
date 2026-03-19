import { useEffect, useRef, useCallback } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from '../../stores/appStore';
import type { DiagnosticProblem, OutputEntry } from '../../types';
import '@xterm/xterm/css/xterm.css';

function parseTerminalLine(line: string): DiagnosticProblem | null {
  // TypeScript pattern
  const tsMatch = line.match(/^(.+?)\((\d+),(\d+)\):\s*(error|warning)\s+(TS\d+):\s*(.+)$/);
  if (tsMatch) {
    return {
      file: tsMatch[1],
      line: parseInt(tsMatch[2]),
      column: parseInt(tsMatch[3]),
      severity: tsMatch[4] as 'error' | 'warning',
      message: tsMatch[6],
      source: 'typescript',
      code: tsMatch[5],
    };
  }

  // Rust pattern: error[E0308]: message --> file:line:col
  const rustMatch = line.match(/^\s*(error|warning)(?:\[([A-Z]\d+)\])?:\s*(.+?)(?:\s*-->\s*(.+?):(\d+):(\d+))?$/);
  if (rustMatch && rustMatch[4]) {
    return {
      file: rustMatch[4],
      line: parseInt(rustMatch[5]),
      column: parseInt(rustMatch[6]),
      severity: rustMatch[1] as 'error' | 'warning',
      message: rustMatch[3],
      source: 'rustc',
      code: rustMatch[2] || undefined,
    };
  }

  // ESLint / generic pattern: file:line:col: severity message
  const genericMatch = line.match(/^(.+?):(\d+):(\d+):\s*(error|warning|info)[:.]?\s*(.+)$/);
  if (genericMatch) {
    return {
      file: genericMatch[1],
      line: parseInt(genericMatch[2]),
      column: parseInt(genericMatch[3]),
      severity: genericMatch[4] as 'error' | 'warning' | 'info',
      message: genericMatch[5],
      source: 'lint',
    };
  }

  return null;
}

function ProblemsView({ problems, onOpenFile }: { problems: DiagnosticProblem[]; onOpenFile: (file: string, line: number) => void }) {
  // Group by file
  const grouped = problems.reduce<Record<string, DiagnosticProblem[]>>((acc, p) => {
    if (!acc[p.file]) acc[p.file] = [];
    acc[p.file].push(p);
    return acc;
  }, {});

  if (problems.length === 0) {
    return (
      <div className="problems-view">
        <div className="problems-empty">No problems detected</div>
      </div>
    );
  }

  return (
    <div className="problems-view">
      {Object.entries(grouped).map(([file, fileProblems]) => (
        <div key={file} className="problems-file-group">
          <div className="problems-file-header">
            <span className="problems-file-name">{file.split('/').pop()}</span>
            <span className="problems-file-path">{file}</span>
            <span className="problems-file-count">{fileProblems.length}</span>
          </div>
          {fileProblems.map((p, i) => (
            <div
              key={i}
              className="problems-row"
              onClick={() => onOpenFile(p.file, p.line)}
            >
              <span className={`problems-severity-icon severity-${p.severity}`}>
                {p.severity === 'error' ? '\u2716' : p.severity === 'warning' ? '\u26A0' : '\u2139'}
              </span>
              <span className="problems-message">{p.message}</span>
              {p.code && <span className="problems-code">{p.code}</span>}
              <span className="problems-location">{p.source} [{p.line}:{p.column}]</span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

function OutputView({ entries }: { entries: OutputEntry[] }) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [entries]);

  if (entries.length === 0) {
    return (
      <div className="output-view">
        <div className="output-empty">No output</div>
      </div>
    );
  }

  return (
    <div className="output-view" ref={containerRef}>
      {entries.map((entry, i) => (
        <div key={i} className={`output-line output-${entry.level}`}>
          <span className="output-timestamp">
            {new Date(entry.timestamp).toLocaleTimeString()}
          </span>
          <span className="output-source">[{entry.source}]</span>
          <span className="output-message">{entry.message}</span>
        </div>
      ))}
    </div>
  );
}

export default function TerminalPanel() {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const {
    bottomPanelTab,
    setBottomPanelTab,
    problems,
    addProblem,
    outputLog,
    addOutputEntry,
    openFile,
  } = useAppStore();

  const handleOpenFile = useCallback((file: string, _line: number) => {
    invoke<string>('read_file', { path: file })
      .then((content) => {
        const name = file.split('/').pop() || file;
        const ext = name.split('.').pop() || '';
        openFile({ path: file, name, content, language: ext, modified: false });
      })
      .catch(() => {});
  }, [openFile]);

  const initTerminal = useCallback(async () => {
    if (!terminalRef.current || xtermRef.current) return;

    const xterm = new XTerm({
      theme: {
        background: 'var(--bg-primary)',
        foreground: 'var(--text-primary)',
        cursor: '#aeafad',
        selectionBackground: '#264f78',
        black: 'var(--bg-primary)',
        red: '#f44747',
        green: '#4ec9b0',
        yellow: '#dcdcaa',
        blue: '#569cd6',
        magenta: '#c586c0',
        cyan: '#4ec9b0',
        white: '#d4d4d4',
        brightBlack: '#808080',
        brightRed: '#f44747',
        brightGreen: '#4ec9b0',
        brightYellow: '#dcdcaa',
        brightBlue: '#569cd6',
        brightMagenta: '#c586c0',
        brightCyan: '#4ec9b0',
        brightWhite: '#e7e7e7',
      },
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
      cursorBlink: true,
      cursorStyle: 'bar',
      scrollback: 10000,
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();
    xterm.loadAddon(fitAddon);
    xterm.loadAddon(webLinksAddon);

    xterm.open(terminalRef.current);
    fitAddon.fit();

    xtermRef.current = xterm;
    fitAddonRef.current = fitAddon;

    // Start PTY backend
    try {
      await invoke('spawn_terminal', {
        id: 'main',
        rows: xterm.rows,
        cols: xterm.cols,
      });
    } catch (err) {
      xterm.writeln('\x1b[33m[LocalCode Terminal]\x1b[0m');
      xterm.writeln('\x1b[90mTerminal backend not available. Using fallback.\x1b[0m');
      xterm.writeln('');
    }

    // Send input to backend
    xterm.onData((data) => {
      invoke('write_terminal', { id: 'main', data }).catch(() => {});
    });

    xterm.onResize(({ rows, cols }) => {
      invoke('resize_terminal', { id: 'main', rows, cols }).catch(() => {});
    });

    // Listen for PTY output and parse for errors
    listen<string>('terminal-output', (event) => {
      xterm.write(event.payload);

      // Parse for problems and output
      const lines = event.payload.split('\n');
      for (const line of lines) {
        // Strip ANSI codes for parsing
        const clean = line.replace(/\x1b\[[0-9;]*[a-zA-Z]/g, '').trim();
        if (!clean) continue;

        const problem = parseTerminalLine(clean);
        if (problem) {
          addProblem(problem);
        }

        // Add to output log
        if (clean.length > 2) {
          const level = clean.toLowerCase().includes('error') ? 'error'
            : clean.toLowerCase().includes('warn') ? 'warn'
            : 'info';
          addOutputEntry({
            timestamp: Date.now(),
            source: 'terminal',
            level: level as 'error' | 'warn' | 'info',
            message: clean,
          });
        }
      }
    });
  }, [addProblem, addOutputEntry]);

  useEffect(() => {
    initTerminal();

    const observer = new ResizeObserver(() => {
      if (bottomPanelTab === 'terminal') {
        fitAddonRef.current?.fit();
      }
    });

    if (terminalRef.current) {
      observer.observe(terminalRef.current);
    }

    return () => {
      observer.disconnect();
    };
  }, [initTerminal]);

  // Re-fit terminal when switching back to terminal tab
  useEffect(() => {
    if (bottomPanelTab === 'terminal') {
      setTimeout(() => fitAddonRef.current?.fit(), 50);
    }
  }, [bottomPanelTab]);

  const errorCount = problems.filter((p) => p.severity === 'error').length;
  const warningCount = problems.filter((p) => p.severity === 'warning').length;

  return (
    <div className="terminal-panel" style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div className="terminal-header">
        <span
          className={`tab ${bottomPanelTab === 'terminal' ? 'active' : ''}`}
          onClick={() => setBottomPanelTab('terminal')}
        >
          Terminal
        </span>
        <span
          className={`tab ${bottomPanelTab === 'problems' ? 'active' : ''}`}
          onClick={() => setBottomPanelTab('problems')}
        >
          Problems
          {(errorCount + warningCount) > 0 && (
            <span className="tab-badge">
              {errorCount > 0 && <span className="tab-badge-error">{errorCount}</span>}
              {warningCount > 0 && <span className="tab-badge-warning">{warningCount}</span>}
            </span>
          )}
        </span>
        <span
          className={`tab ${bottomPanelTab === 'output' ? 'active' : ''}`}
          onClick={() => setBottomPanelTab('output')}
        >
          Output
        </span>
        <div className="actions">
          {bottomPanelTab === 'terminal' && (
            <>
              <button className="action-btn" title="New Terminal" onClick={() => {
                invoke('spawn_terminal', { id: `term-${Date.now()}`, rows: 24, cols: 80 }).catch(() => {});
              }}>
                <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M14 7v1H8v6H7V8H1V7h6V1h1v6h6z" />
                </svg>
              </button>
              <button className="action-btn" title="Clear" onClick={() => xtermRef.current?.clear()}>
                <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M10 12.6l.7.7 1.6-1.6 1.6 1.6.8-.7L13 11l1.7-1.6-.8-.8-1.6 1.7-1.6-1.7-.7.8L11.6 11 10 12.6zM1 4h14V3H1v1zm0 3h14V6H1v1zm0 3h8V9H1v1zm0 3h8v-1H1v1z" />
                </svg>
              </button>
            </>
          )}
          {bottomPanelTab === 'problems' && (
            <button className="action-btn" title="Clear Problems" onClick={() => useAppStore.getState().clearProblems()}>
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                <path d="M10 12.6l.7.7 1.6-1.6 1.6 1.6.8-.7L13 11l1.7-1.6-.8-.8-1.6 1.7-1.6-1.7-.7.8L11.6 11 10 12.6zM1 4h14V3H1v1zm0 3h14V6H1v1zm0 3h8V9H1v1zm0 3h8v-1H1v1z" />
              </svg>
            </button>
          )}
          {bottomPanelTab === 'output' && (
            <button className="action-btn" title="Clear Output" onClick={() => useAppStore.getState().clearOutput()}>
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                <path d="M10 12.6l.7.7 1.6-1.6 1.6 1.6.8-.7L13 11l1.7-1.6-.8-.8-1.6 1.7-1.6-1.7-.7.8L11.6 11 10 12.6zM1 4h14V3H1v1zm0 3h14V6H1v1zm0 3h8V9H1v1zm0 3h8v-1H1v1z" />
              </svg>
            </button>
          )}
        </div>
      </div>
      {/* Terminal is always mounted, hidden when not active */}
      <div
        className="terminal-body"
        ref={terminalRef}
        style={{ display: bottomPanelTab === 'terminal' ? 'block' : 'none' }}
      />
      {bottomPanelTab === 'problems' && (
        <ProblemsView problems={problems} onOpenFile={handleOpenFile} />
      )}
      {bottomPanelTab === 'output' && (
        <OutputView entries={outputLog} />
      )}
    </div>
  );
}
