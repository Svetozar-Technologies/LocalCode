import { useEffect, useRef, useCallback } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import '@xterm/xterm/css/xterm.css';

export default function TerminalPanel() {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  const initTerminal = useCallback(async () => {
    if (!terminalRef.current || xtermRef.current) return;

    const xterm = new XTerm({
      theme: {
        background: '#1e1e1e',
        foreground: '#cccccc',
        cursor: '#aeafad',
        selectionBackground: '#264f78',
        black: '#1e1e1e',
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

    // Listen for PTY output
    listen<string>('terminal-output', (event) => {
      xterm.write(event.payload);
    });
  }, []);

  useEffect(() => {
    initTerminal();

    const observer = new ResizeObserver(() => {
      fitAddonRef.current?.fit();
    });

    if (terminalRef.current) {
      observer.observe(terminalRef.current);
    }

    return () => {
      observer.disconnect();
    };
  }, [initTerminal]);

  return (
    <div className="terminal-panel" style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div className="terminal-header">
        <span className="tab active">Terminal</span>
        <span className="tab">Problems</span>
        <span className="tab">Output</span>
        <div className="actions">
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
        </div>
      </div>
      <div className="terminal-body" ref={terminalRef} />
    </div>
  );
}
