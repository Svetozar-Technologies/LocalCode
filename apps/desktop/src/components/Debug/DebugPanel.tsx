import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';
import Variables from './Variables';
import CallStack from './CallStack';

type DebugState = 'idle' | 'running' | 'stopped' | 'initializing';

interface StackFrame {
  id: number;
  name: string;
  file?: string;
  line: number;
  column: number;
}

interface VariableItem {
  name: string;
  value: string;
  type?: string;
  variablesReference: number;
}

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    height: '100%',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    color: '#cccccc',
    fontSize: 13,
  } as React.CSSProperties,
  toolbar: {
    display: 'flex',
    alignItems: 'center',
    padding: '6px 12px',
    gap: 4,
    borderBottom: '1px solid #3c3c3c',
    background: '#252526',
  } as React.CSSProperties,
  toolbarButton: {
    background: 'none',
    border: 'none',
    color: '#cccccc',
    cursor: 'pointer',
    padding: '4px 6px',
    borderRadius: 3,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: 14,
    opacity: 0.8,
  } as React.CSSProperties,
  toolbarButtonDisabled: {
    opacity: 0.3,
    cursor: 'default',
  } as React.CSSProperties,
  statusBadge: {
    fontSize: 11,
    padding: '2px 8px',
    borderRadius: 10,
    marginLeft: 'auto',
    fontWeight: 500,
  } as React.CSSProperties,
  sections: {
    flex: 1,
    overflow: 'auto',
  } as React.CSSProperties,
  section: {
    borderBottom: '1px solid #3c3c3c',
  } as React.CSSProperties,
  sectionHeader: {
    display: 'flex',
    alignItems: 'center',
    padding: '6px 12px',
    fontSize: 11,
    fontWeight: 700,
    textTransform: 'uppercase' as const,
    color: '#969696',
    cursor: 'pointer',
    userSelect: 'none' as const,
    gap: 6,
  } as React.CSSProperties,
  sectionContent: {
    padding: '0 12px 8px',
  } as React.CSSProperties,
  launchConfig: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 8,
    padding: '12px',
  } as React.CSSProperties,
  input: {
    background: '#3c3c3c',
    border: '1px solid #3c3c3c',
    borderRadius: 3,
    color: '#cccccc',
    padding: '5px 8px',
    fontSize: 12,
    outline: 'none',
    width: '100%',
  } as React.CSSProperties,
  label: {
    fontSize: 11,
    color: '#969696',
    marginBottom: 2,
  } as React.CSSProperties,
  startButton: {
    background: '#4ec9b0',
    color: '#1e1e1e',
    border: 'none',
    borderRadius: 3,
    padding: '6px 16px',
    fontSize: 12,
    fontWeight: 600,
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 6,
  } as React.CSSProperties,
  outputArea: {
    padding: '8px 12px',
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    fontSize: 12,
    lineHeight: 1.6,
    maxHeight: 200,
    overflow: 'auto',
    background: '#1e1e1e',
  } as React.CSSProperties,
  empty: {
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 40,
    color: '#6a6a6a',
    fontSize: 13,
    gap: 12,
    textAlign: 'center' as const,
  } as React.CSSProperties,
};

export default function DebugPanel() {
  const { projectPath } = useAppStore();
  const [state, setState] = useState<DebugState>('idle');
  const [program, setProgram] = useState('');
  const [debugType, setDebugType] = useState('python');
  const [stackFrames, setStackFrames] = useState<StackFrame[]>([]);
  const [variables, setVariables] = useState<VariableItem[]>([]);
  const [output, setOutput] = useState<string[]>([]);
  const [expandedSections, setExpandedSections] = useState({
    variables: true,
    callStack: true,
    breakpoints: true,
    output: true,
  });

  const toggleSection = (section: keyof typeof expandedSections) => {
    setExpandedSections((prev) => ({
      ...prev,
      [section]: !prev[section],
    }));
  };

  const handleStart = useCallback(async () => {
    if (!projectPath || !program) return;
    setState('initializing');
    setOutput([]);
    setStackFrames([]);
    setVariables([]);

    try {
      await invoke('debug_start', {
        path: projectPath,
        program,
        adapterType: debugType,
      });
      setState('running');
      setOutput((prev) => [...prev, `[Debug] Started debugging ${program}`]);
    } catch (err) {
      setState('idle');
      setOutput((prev) => [...prev, `[Error] ${err}`]);
    }
  }, [projectPath, program, debugType]);

  const handleStop = useCallback(async () => {
    try {
      await invoke('debug_stop');
    } catch (_) {}
    setState('idle');
    setOutput((prev) => [...prev, '[Debug] Session ended']);
  }, []);

  const handleContinue = useCallback(async () => {
    try {
      await invoke('debug_continue');
      setState('running');
    } catch (err) {
      setOutput((prev) => [...prev, `[Error] ${err}`]);
    }
  }, []);

  const handleStepOver = useCallback(async () => {
    try {
      await invoke('debug_step_over');
    } catch (err) {
      setOutput((prev) => [...prev, `[Error] ${err}`]);
    }
  }, []);

  const handleStepInto = useCallback(async () => {
    try {
      await invoke('debug_step_into');
    } catch (err) {
      setOutput((prev) => [...prev, `[Error] ${err}`]);
    }
  }, []);

  const handleStepOut = useCallback(async () => {
    try {
      await invoke('debug_step_out');
    } catch (err) {
      setOutput((prev) => [...prev, `[Error] ${err}`]);
    }
  }, []);

  const handlePause = useCallback(async () => {
    try {
      await invoke('debug_pause');
      setState('stopped');
    } catch (err) {
      setOutput((prev) => [...prev, `[Error] ${err}`]);
    }
  }, []);

  const isStopped = state === 'stopped';
  const isRunning = state === 'running' || state === 'stopped';

  const statusColors: Record<DebugState, string> = {
    idle: '#6a6a6a',
    initializing: '#dcdcaa',
    running: '#4ec9b0',
    stopped: '#ce9178',
  };

  if (state === 'idle') {
    return (
      <div style={styles.container}>
        <div style={styles.toolbar}>
          <span style={{ fontWeight: 600 }}>Debug</span>
          <span
            style={{
              ...styles.statusBadge,
              color: statusColors.idle,
              background: '#2a2d2e',
            }}
          >
            Not started
          </span>
        </div>
        <div style={styles.launchConfig}>
          <div>
            <div style={styles.label}>Debug Type</div>
            <select
              style={{ ...styles.input, cursor: 'pointer' }}
              value={debugType}
              onChange={(e) => setDebugType(e.target.value)}
            >
              <option value="python">Python (debugpy)</option>
              <option value="node">Node.js</option>
              <option value="lldb">LLDB (Rust/C/C++)</option>
            </select>
          </div>
          <div>
            <div style={styles.label}>Program</div>
            <input
              style={styles.input}
              placeholder="e.g., main.py, app.js"
              value={program}
              onChange={(e) => setProgram(e.target.value)}
              onFocus={(e) => {
                (e.target as HTMLInputElement).style.borderColor = '#007acc';
              }}
              onBlur={(e) => {
                (e.target as HTMLInputElement).style.borderColor = '#3c3c3c';
              }}
            />
          </div>
          <button
            style={{
              ...styles.startButton,
              opacity: program ? 1 : 0.5,
            }}
            onClick={handleStart}
            disabled={!program}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 2l10 6-10 6V2z" />
            </svg>
            Start Debugging
          </button>
        </div>
        {output.length > 0 && (
          <div style={styles.outputArea}>
            {output.map((line, i) => (
              <div key={i}>{line}</div>
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <div style={styles.container}>
      <div style={styles.toolbar}>
        <button
          style={{
            ...styles.toolbarButton,
            ...(isStopped ? {} : styles.toolbarButtonDisabled),
          }}
          onClick={handleContinue}
          disabled={!isStopped}
          title="Continue (F5)"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="#4ec9b0">
            <path d="M4 2l10 6-10 6V2z" />
          </svg>
        </button>
        <button
          style={{
            ...styles.toolbarButton,
            ...(isStopped ? {} : styles.toolbarButtonDisabled),
          }}
          onClick={handleStepOver}
          disabled={!isStopped}
          title="Step Over (F10)"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="#569cd6">
            <path d="M14.25 5.75a1.5 1.5 0 00-1.5-1.5h-4a1 1 0 000 2h3v3a1 1 0 002 0v-3.5z" />
            <path d="M10.75 7.25l-2-2 2-2" stroke="#569cd6" strokeWidth="1.5" fill="none" />
          </svg>
        </button>
        <button
          style={{
            ...styles.toolbarButton,
            ...(isStopped ? {} : styles.toolbarButtonDisabled),
          }}
          onClick={handleStepInto}
          disabled={!isStopped}
          title="Step Into (F11)"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="#569cd6">
            <path d="M8 2v8M5 7l3 3 3-3" stroke="#569cd6" strokeWidth="1.5" fill="none" />
            <circle cx="8" cy="13" r="1.5" fill="#569cd6" />
          </svg>
        </button>
        <button
          style={{
            ...styles.toolbarButton,
            ...(isStopped ? {} : styles.toolbarButtonDisabled),
          }}
          onClick={handleStepOut}
          disabled={!isStopped}
          title="Step Out (Shift+F11)"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="#569cd6">
            <path d="M8 12V4M5 7l3-3 3 3" stroke="#569cd6" strokeWidth="1.5" fill="none" />
            <circle cx="8" cy="14" r="1.5" fill="#569cd6" />
          </svg>
        </button>
        <button
          style={{
            ...styles.toolbarButton,
            ...(state === 'running' ? {} : styles.toolbarButtonDisabled),
          }}
          onClick={handlePause}
          disabled={state !== 'running'}
          title="Pause (F6)"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="#dcdcaa">
            <rect x="4" y="3" width="3" height="10" />
            <rect x="9" y="3" width="3" height="10" />
          </svg>
        </button>
        <button
          style={{
            ...styles.toolbarButton,
            ...(isRunning ? {} : styles.toolbarButtonDisabled),
          }}
          onClick={handleStop}
          disabled={!isRunning}
          title="Stop (Shift+F5)"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="#f44747">
            <rect x="3" y="3" width="10" height="10" rx="1" />
          </svg>
        </button>

        <span
          style={{
            ...styles.statusBadge,
            color: statusColors[state],
            background: '#2a2d2e',
          }}
        >
          {state === 'running' ? 'Running' : state === 'stopped' ? 'Paused' : 'Initializing'}
        </span>
      </div>

      <div style={styles.sections}>
        <div style={styles.section}>
          <div
            style={styles.sectionHeader}
            onClick={() => toggleSection('variables')}
          >
            <span>{expandedSections.variables ? '\u25BC' : '\u25B6'}</span>
            Variables
          </div>
          {expandedSections.variables && (
            <div style={styles.sectionContent}>
              <Variables variables={variables} />
            </div>
          )}
        </div>

        <div style={styles.section}>
          <div
            style={styles.sectionHeader}
            onClick={() => toggleSection('callStack')}
          >
            <span>{expandedSections.callStack ? '\u25BC' : '\u25B6'}</span>
            Call Stack
          </div>
          {expandedSections.callStack && (
            <div style={styles.sectionContent}>
              <CallStack frames={stackFrames} />
            </div>
          )}
        </div>

        <div style={styles.section}>
          <div
            style={styles.sectionHeader}
            onClick={() => toggleSection('output')}
          >
            <span>{expandedSections.output ? '\u25BC' : '\u25B6'}</span>
            Debug Console
          </div>
          {expandedSections.output && (
            <div style={styles.outputArea}>
              {output.length === 0 ? (
                <span style={{ color: '#6a6a6a' }}>No output yet</span>
              ) : (
                output.map((line, i) => (
                  <div
                    key={i}
                    style={{
                      color: line.startsWith('[Error]')
                        ? '#f44747'
                        : line.startsWith('[Debug]')
                        ? '#569cd6'
                        : '#cccccc',
                    }}
                  >
                    {line}
                  </div>
                ))
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
