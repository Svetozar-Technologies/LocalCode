import { useState, useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { editor } from 'monaco-editor';

interface InlineEditProps {
  editorInstance: editor.IStandaloneCodeEditor | null;
}

interface InlineEditState {
  visible: boolean;
  selectedText: string;
  instruction: string;
  originalCode: string;
  proposedCode: string;
  status: 'idle' | 'loading' | 'preview' | 'error';
  error: string;
  selectionRange: any | null;
}

const styles = {
  overlay: {
    position: 'absolute' as const,
    zIndex: 100,
    background: 'var(--bg-secondary)',
    border: '1px solid #007acc',
    borderRadius: 6,
    boxShadow: '0 8px 24px rgba(0, 0, 0, 0.4)',
    width: 420,
    overflow: 'hidden',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  header: {
    display: 'flex',
    alignItems: 'center',
    padding: '8px 12px',
    background: 'var(--bg-primary)',
    borderBottom: '1px solid var(--border-color)',
    fontSize: 12,
    fontWeight: 600,
    color: 'var(--text-primary)',
    gap: 8,
  } as React.CSSProperties,
  badge: {
    background: '#007acc',
    color: '#ffffff',
    padding: '1px 6px',
    borderRadius: 8,
    fontSize: 10,
    fontWeight: 600,
  } as React.CSSProperties,
  inputArea: {
    padding: 10,
  } as React.CSSProperties,
  input: {
    width: '100%',
    background: 'var(--border-color)',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '8px 10px',
    fontSize: 13,
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    outline: 'none',
    resize: 'none' as const,
    minHeight: 36,
    maxHeight: 100,
  } as React.CSSProperties,
  inputFocused: {
    borderColor: '#007acc',
  },
  hint: {
    fontSize: 11,
    color: 'var(--text-muted)',
    marginTop: 6,
    display: 'flex',
    alignItems: 'center',
    gap: 4,
  } as React.CSSProperties,
  kbd: {
    background: 'var(--bg-tertiary)',
    border: '1px solid var(--border-color)',
    borderRadius: 3,
    padding: '1px 5px',
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    fontSize: 10,
  } as React.CSSProperties,
  previewArea: {
    padding: '0 10px 10px',
  } as React.CSSProperties,
  diffContainer: {
    background: 'var(--bg-primary)',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    overflow: 'auto',
    maxHeight: 200,
    fontSize: 12,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
  } as React.CSSProperties,
  diffLine: {
    padding: '1px 8px',
    whiteSpace: 'pre' as const,
    lineHeight: '18px',
  } as React.CSSProperties,
  diffRemoved: {
    background: 'rgba(244, 71, 71, 0.15)',
    color: '#f44747',
  } as React.CSSProperties,
  diffAdded: {
    background: 'rgba(78, 201, 176, 0.15)',
    color: '#4ec9b0',
  } as React.CSSProperties,
  diffContext: {
    color: 'var(--text-muted)',
  } as React.CSSProperties,
  actions: {
    display: 'flex',
    justifyContent: 'flex-end',
    gap: 6,
    padding: '8px 10px',
    borderTop: '1px solid var(--border-color)',
    background: 'var(--bg-primary)',
  } as React.CSSProperties,
  acceptButton: {
    background: '#4ec9b0',
    border: 'none',
    borderRadius: 4,
    color: 'var(--bg-primary)',
    padding: '5px 14px',
    cursor: 'pointer',
    fontSize: 12,
    fontWeight: 600,
  } as React.CSSProperties,
  rejectButton: {
    background: 'none',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '5px 14px',
    cursor: 'pointer',
    fontSize: 12,
  } as React.CSSProperties,
  loadingBar: {
    height: 2,
    background: '#007acc',
    animation: 'inline-edit-loading 1.5s ease-in-out infinite',
  } as React.CSSProperties,
  errorText: {
    color: '#f44747',
    fontSize: 12,
    padding: '8px 10px',
  } as React.CSSProperties,
};

function computeSimpleDiff(original: string, modified: string): Array<{ type: 'added' | 'removed' | 'context'; text: string }> {
  const origLines = original.split('\n');
  const modLines = modified.split('\n');
  const result: Array<{ type: 'added' | 'removed' | 'context'; text: string }> = [];

  const maxLen = Math.max(origLines.length, modLines.length);
  let i = 0;
  let j = 0;

  while (i < origLines.length || j < modLines.length) {
    if (i < origLines.length && j < modLines.length && origLines[i] === modLines[j]) {
      result.push({ type: 'context', text: origLines[i] });
      i++;
      j++;
    } else {
      // Find where they sync again
      let syncI = i;
      let syncJ = j;
      let found = false;

      for (let look = 1; look < maxLen && !found; look++) {
        for (let di = 0; di <= look; di++) {
          const dj = look - di;
          if (i + di < origLines.length && j + dj < modLines.length && origLines[i + di] === modLines[j + dj]) {
            syncI = i + di;
            syncJ = j + dj;
            found = true;
            break;
          }
        }
      }

      if (!found) {
        syncI = origLines.length;
        syncJ = modLines.length;
      }

      while (i < syncI) {
        result.push({ type: 'removed', text: origLines[i] });
        i++;
      }
      while (j < syncJ) {
        result.push({ type: 'added', text: modLines[j] });
        j++;
      }
    }
  }

  return result;
}

export default function InlineEdit({ editorInstance }: InlineEditProps) {
  const [state, setState] = useState<InlineEditState>({
    visible: false,
    selectedText: '',
    instruction: '',
    originalCode: '',
    proposedCode: '',
    status: 'idle',
    error: '',
    selectionRange: null,
  });

  const inputRef = useRef<HTMLTextAreaElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const [inputFocused, setInputFocused] = useState(false);

  // Register Cmd+K keybinding
  useEffect(() => {
    if (!editorInstance) return;

    const monaco = (window as any).monaco;
    if (!monaco) return;

    editorInstance.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK,
      () => {
        const selection = editorInstance.getSelection();
        if (!selection || selection.isEmpty()) return;

        const model = editorInstance.getModel();
        if (!model) return;

        const selectedText = model.getValueInRange(selection);
        if (!selectedText.trim()) return;

        // Calculate position based on selection
        const topPosition = editorInstance.getTopForLineNumber(selection.startLineNumber);
        const scrollTop = editorInstance.getScrollTop();
        const layoutInfo = editorInstance.getLayoutInfo();

        setPosition({
          top: Math.max(0, topPosition - scrollTop + 24),
          left: Math.min(layoutInfo.width - 440, layoutInfo.contentLeft + 20),
        });

        setState({
          visible: true,
          selectedText,
          instruction: '',
          originalCode: selectedText,
          proposedCode: '',
          status: 'idle',
          error: '',
          selectionRange: selection,
        });

        setTimeout(() => inputRef.current?.focus(), 50);
      }
    );

    return () => {
      // disposable is void for addCommand, no cleanup needed
    };
  }, [editorInstance]);

  // Listen for LLM streaming response (reuse llm-chat events with inline-edit ID prefix)
  useEffect(() => {
    const unlisten = listen<{ id: string; chunk: string }>('llm-chat-chunk', (event) => {
      if (!event.payload.id.startsWith('inline-edit-')) return;
      setState((prev) => ({
        ...prev,
        proposedCode: prev.proposedCode + event.payload.chunk,
      }));
    });

    const unlistenDone = listen<{ id: string }>('llm-chat-done', (event) => {
      if (!event.payload.id.startsWith('inline-edit-')) return;
      setState((prev) => ({
        ...prev,
        status: prev.proposedCode ? 'preview' : 'error',
        error: prev.proposedCode ? '' : 'No response received from model.',
      }));
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenDone.then((fn) => fn());
    };
  }, []);

  const handleSubmit = useCallback(async () => {
    if (!state.instruction.trim() || state.status === 'loading') return;

    setState((prev) => ({
      ...prev,
      status: 'loading',
      proposedCode: '',
      error: '',
    }));

    const requestId = `inline-edit-${Date.now()}`;
    const filePath = editorInstance?.getModel()?.uri.path || '';

    try {
      // Use llm_chat with a specialized prompt for inline editing
      await invoke('llm_chat', {
        responseId: requestId,
        messages: [
          {
            role: 'system',
            content: 'You are a code editing assistant. The user will give you a piece of code and an instruction. Output ONLY the modified code, with no explanations, no markdown fences, no commentary. Just the raw code.',
          },
          {
            role: 'user',
            content: `File: ${filePath}\n\nOriginal code:\n${state.selectedText}\n\nInstruction: ${state.instruction}\n\nModified code:`,
          },
        ],
        context: '',
      });
    } catch (err) {
      setState((prev) => ({
        ...prev,
        status: 'error',
        error: `Failed to generate edit: ${err}`,
      }));
    }
  }, [state.instruction, state.selectedText, state.status, editorInstance]);

  const handleAccept = useCallback(() => {
    if (!editorInstance || !state.selectionRange || !state.proposedCode) return;

    editorInstance.executeEdits('inline-edit', [
      { range: state.selectionRange, text: state.proposedCode },
    ]);

    setState({
      visible: false,
      selectedText: '',
      instruction: '',
      originalCode: '',
      proposedCode: '',
      status: 'idle',
      error: '',
      selectionRange: null,
    });

    editorInstance.focus();
  }, [editorInstance, state.selectionRange, state.proposedCode]);

  const handleReject = useCallback(() => {
    setState({
      visible: false,
      selectedText: '',
      instruction: '',
      originalCode: '',
      proposedCode: '',
      status: 'idle',
      error: '',
      selectionRange: null,
    });

    editorInstance?.focus();
  }, [editorInstance]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleReject();
      } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        if (state.status === 'preview') {
          handleAccept();
        } else if (state.status === 'idle') {
          handleSubmit();
        }
      } else if (e.key === 'Enter' && !e.shiftKey && state.status === 'idle') {
        e.preventDefault();
        handleSubmit();
      }
    },
    [handleReject, handleAccept, handleSubmit, state.status]
  );

  if (!state.visible) return null;

  const diffLines = state.proposedCode
    ? computeSimpleDiff(state.originalCode, state.proposedCode)
    : [];

  return (
    <div
      ref={overlayRef}
      style={{
        ...styles.overlay,
        top: position.top,
        left: position.left,
      }}
    >
      <div style={styles.header}>
        <svg width="14" height="14" viewBox="0 0 16 16" fill="#007acc">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93z" />
        </svg>
        <span>Inline Edit</span>
        <span style={styles.badge}>AI</span>
        <span
          style={{ marginLeft: 'auto', cursor: 'pointer', color: 'var(--text-muted)', fontSize: 14 }}
          onClick={handleReject}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 8.707l3.646 3.647.708-.707L8.707 8l3.647-3.646-.707-.708L8 7.293 4.354 3.646l-.707.708L7.293 8l-3.646 3.646.707.708L8 8.707z" />
          </svg>
        </span>
      </div>

      {state.status === 'loading' && (
        <div style={{ overflow: 'hidden' }}>
          <div
            style={{
              height: 2,
              background: 'linear-gradient(90deg, transparent, #007acc, transparent)',
              backgroundSize: '200% 100%',
              animation: 'none',
            }}
          >
            <div
              style={{
                width: '40%',
                height: '100%',
                background: '#007acc',
                animation: 'shimmer 1.5s ease-in-out infinite',
              }}
            />
          </div>
        </div>
      )}

      <div style={styles.inputArea}>
        <textarea
          ref={inputRef}
          style={{
            ...styles.input,
            ...(inputFocused ? { borderColor: '#007acc' } : {}),
          }}
          placeholder="Describe the edit you want to make..."
          value={state.instruction}
          onChange={(e) => setState((prev) => ({ ...prev, instruction: e.target.value }))}
          onKeyDown={handleKeyDown}
          onFocus={() => setInputFocused(true)}
          onBlur={() => setInputFocused(false)}
          disabled={state.status === 'loading'}
          rows={1}
        />
        <div style={styles.hint}>
          <kbd style={styles.kbd}>Enter</kbd>
          <span>to submit</span>
          <kbd style={styles.kbd}>Esc</kbd>
          <span>to cancel</span>
        </div>
      </div>

      {state.status === 'error' && (
        <div style={styles.errorText}>{state.error}</div>
      )}

      {(state.status === 'preview' || (state.status === 'loading' && state.proposedCode)) && diffLines.length > 0 && (
        <div style={styles.previewArea}>
          <div style={{ fontSize: 11, color: 'var(--text-secondary)', marginBottom: 4, fontWeight: 600 }}>
            Preview
          </div>
          <div style={styles.diffContainer}>
            {diffLines.map((line, i) => (
              <div
                key={i}
                style={{
                  ...styles.diffLine,
                  ...(line.type === 'removed' ? styles.diffRemoved : {}),
                  ...(line.type === 'added' ? styles.diffAdded : {}),
                  ...(line.type === 'context' ? styles.diffContext : {}),
                }}
              >
                <span style={{ display: 'inline-block', width: 14, color: 'var(--text-muted)' }}>
                  {line.type === 'removed' ? '-' : line.type === 'added' ? '+' : ' '}
                </span>
                {line.text}
              </div>
            ))}
          </div>
        </div>
      )}

      {state.status === 'preview' && (
        <div style={styles.actions}>
          <button
            style={styles.rejectButton}
            onClick={handleReject}
            onMouseEnter={(e) => { (e.target as HTMLElement).style.borderColor = 'var(--text-secondary)'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.borderColor = 'var(--border-color)'; }}
          >
            Reject
          </button>
          <button
            style={styles.acceptButton}
            onClick={handleAccept}
            onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#3dd1a6'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.background = '#4ec9b0'; }}
          >
            Accept
          </button>
        </div>
      )}
    </div>
  );
}
