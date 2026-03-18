import { useState, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from '../../stores/appStore';
import FileChanges from './FileChanges';

export interface FileChange {
  path: string;
  originalContent: string;
  proposedContent: string;
  status: 'pending' | 'accepted' | 'rejected';
  language: string;
}

type ComposerStatus = 'idle' | 'composing' | 'reviewing' | 'applying' | 'done' | 'error';

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    height: '100%',
    background: '#1e1e1e',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  header: {
    display: 'flex',
    alignItems: 'center',
    padding: '10px 16px',
    borderBottom: '1px solid #3c3c3c',
    background: '#252526',
    gap: 8,
  } as React.CSSProperties,
  title: {
    fontSize: 13,
    fontWeight: 600,
    color: '#cccccc',
  } as React.CSSProperties,
  badge: {
    background: '#c586c0',
    color: '#ffffff',
    padding: '1px 8px',
    borderRadius: 8,
    fontSize: 10,
    fontWeight: 600,
  } as React.CSSProperties,
  statusBadge: {
    fontSize: 10,
    padding: '2px 8px',
    borderRadius: 8,
    fontWeight: 600,
    marginLeft: 'auto',
  } as React.CSSProperties,
  inputArea: {
    padding: 16,
    borderBottom: '1px solid #3c3c3c',
  } as React.CSSProperties,
  textarea: {
    width: '100%',
    background: '#3c3c3c',
    border: '1px solid #3c3c3c',
    borderRadius: 6,
    color: '#cccccc',
    padding: '10px 12px',
    fontSize: 13,
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    resize: 'vertical' as const,
    outline: 'none',
    minHeight: 80,
    maxHeight: 200,
    lineHeight: 1.5,
  } as React.CSSProperties,
  textareaFocused: {
    borderColor: '#007acc',
  },
  submitRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginTop: 10,
  } as React.CSSProperties,
  submitButton: {
    background: '#007acc',
    border: 'none',
    borderRadius: 4,
    color: '#ffffff',
    padding: '7px 20px',
    cursor: 'pointer',
    fontSize: 13,
    fontWeight: 500,
  } as React.CSSProperties,
  cancelButton: {
    background: 'none',
    border: '1px solid #3c3c3c',
    borderRadius: 4,
    color: '#cccccc',
    padding: '7px 16px',
    cursor: 'pointer',
    fontSize: 12,
  } as React.CSSProperties,
  hint: {
    fontSize: 11,
    color: '#6a6a6a',
  } as React.CSSProperties,
  changesArea: {
    flex: 1,
    overflow: 'auto',
  } as React.CSSProperties,
  actionsBar: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'flex-end',
    gap: 8,
    padding: '10px 16px',
    borderTop: '1px solid #3c3c3c',
    background: '#252526',
  } as React.CSSProperties,
  acceptAllButton: {
    background: '#4ec9b0',
    border: 'none',
    borderRadius: 4,
    color: '#1e1e1e',
    padding: '6px 16px',
    cursor: 'pointer',
    fontSize: 12,
    fontWeight: 600,
  } as React.CSSProperties,
  rejectAllButton: {
    background: 'none',
    border: '1px solid #f44747',
    borderRadius: 4,
    color: '#f44747',
    padding: '6px 16px',
    cursor: 'pointer',
    fontSize: 12,
  } as React.CSSProperties,
  loadingArea: {
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 40,
    gap: 12,
    color: '#969696',
  } as React.CSSProperties,
  spinner: {
    width: 24,
    height: 24,
    border: '2px solid #3c3c3c',
    borderTop: '2px solid #007acc',
    borderRadius: '50%',
    animation: 'spin 0.8s linear infinite',
  } as React.CSSProperties,
  summary: {
    padding: '12px 16px',
    background: '#2d2d2d',
    borderBottom: '1px solid #3c3c3c',
    display: 'flex',
    alignItems: 'center',
    gap: 16,
    fontSize: 12,
    color: '#969696',
  } as React.CSSProperties,
  errorText: {
    color: '#f44747',
    fontSize: 12,
    padding: '12px 16px',
  } as React.CSSProperties,
};

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact',
    py: 'python', rs: 'rust', go: 'go', java: 'java', c: 'c', cpp: 'cpp',
    html: 'html', css: 'css', json: 'json', md: 'markdown',
  };
  return map[ext] || 'plaintext';
}

function getStatusColor(status: ComposerStatus): string {
  switch (status) {
    case 'idle': return '#6a6a6a';
    case 'composing': return '#007acc';
    case 'reviewing': return '#dcdcaa';
    case 'applying': return '#c586c0';
    case 'done': return '#4ec9b0';
    case 'error': return '#f44747';
    default: return '#6a6a6a';
  }
}

function getStatusLabel(status: ComposerStatus): string {
  switch (status) {
    case 'idle': return 'Ready';
    case 'composing': return 'Generating...';
    case 'reviewing': return 'Review Changes';
    case 'applying': return 'Applying...';
    case 'done': return 'Complete';
    case 'error': return 'Error';
    default: return '';
  }
}

export default function Composer() {
  const { projectPath } = useAppStore();
  const [task, setTask] = useState('');
  const [status, setStatus] = useState<ComposerStatus>('idle');
  const [fileChanges, setFileChanges] = useState<FileChange[]>([]);
  const [error, setError] = useState('');
  const [inputFocused, setInputFocused] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleSubmit = useCallback(async () => {
    if (!task.trim() || status === 'composing') return;

    setStatus('composing');
    setFileChanges([]);
    setError('');

    const requestId = `composer-${Date.now()}`;

    // Listen for file change events
    const unlistenChange = await listen<{
      id: string;
      path: string;
      original: string;
      proposed: string;
    }>('composer-file-change', (event) => {
      if (event.payload.id !== requestId) return;
      setFileChanges((prev) => [
        ...prev,
        {
          path: event.payload.path,
          originalContent: event.payload.original,
          proposedContent: event.payload.proposed,
          status: 'pending',
          language: getLanguageFromPath(event.payload.path),
        },
      ]);
    });

    const unlistenDone = await listen<{ id: string }>('composer-done', (event) => {
      if (event.payload.id !== requestId) return;
      setStatus('reviewing');
      unlistenChange();
      unlistenDone();
    });

    const unlistenError = await listen<{ id: string; error: string }>('composer-error', (event) => {
      if (event.payload.id !== requestId) return;
      setStatus('error');
      setError(event.payload.error);
      unlistenChange();
      unlistenDone();
      unlistenError();
    });

    try {
      await invoke('composer_generate', {
        requestId,
        task: task.trim(),
        projectPath: projectPath || '',
      });
    } catch (err) {
      setStatus('error');
      setError(`Composer failed: ${err}`);
      unlistenChange();
      unlistenDone();
      unlistenError();
    }
  }, [task, status, projectPath]);

  const handleAcceptFile = useCallback((path: string) => {
    setFileChanges((prev) =>
      prev.map((fc) => (fc.path === path ? { ...fc, status: 'accepted' as const } : fc))
    );
  }, []);

  const handleRejectFile = useCallback((path: string) => {
    setFileChanges((prev) =>
      prev.map((fc) => (fc.path === path ? { ...fc, status: 'rejected' as const } : fc))
    );
  }, []);

  const handleAcceptAll = useCallback(() => {
    setFileChanges((prev) =>
      prev.map((fc) => (fc.status === 'pending' ? { ...fc, status: 'accepted' as const } : fc))
    );
  }, []);

  const handleRejectAll = useCallback(() => {
    setFileChanges((prev) =>
      prev.map((fc) => (fc.status === 'pending' ? { ...fc, status: 'rejected' as const } : fc))
    );
  }, []);

  const handleApply = useCallback(async () => {
    const acceptedChanges = fileChanges.filter((fc) => fc.status === 'accepted');
    if (acceptedChanges.length === 0) return;

    setStatus('applying');

    try {
      for (const change of acceptedChanges) {
        await invoke('write_file', {
          path: change.path,
          content: change.proposedContent,
        });
      }
      setStatus('done');
    } catch (err) {
      setStatus('error');
      setError(`Failed to apply changes: ${err}`);
    }
  }, [fileChanges]);

  const handleReset = useCallback(() => {
    setTask('');
    setStatus('idle');
    setFileChanges([]);
    setError('');
    textareaRef.current?.focus();
  }, []);

  const pendingCount = fileChanges.filter((fc) => fc.status === 'pending').length;
  const acceptedCount = fileChanges.filter((fc) => fc.status === 'accepted').length;
  const rejectedCount = fileChanges.filter((fc) => fc.status === 'rejected').length;

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="#c586c0">
          <path d="M14.5 2h-13L1 2.5v11l.5.5h13l.5-.5v-11l-.5-.5zM14 13H2V6h12v7zm0-8H2V3h12v2z" />
        </svg>
        <span style={styles.title}>Composer</span>
        <span style={styles.badge}>Multi-File AI</span>
        <span
          style={{
            ...styles.statusBadge,
            background: `${getStatusColor(status)}22`,
            color: getStatusColor(status),
            border: `1px solid ${getStatusColor(status)}44`,
          }}
        >
          {getStatusLabel(status)}
        </span>
      </div>

      {/* Task input */}
      <div style={styles.inputArea}>
        <textarea
          ref={textareaRef}
          style={{
            ...styles.textarea,
            ...(inputFocused ? { borderColor: '#007acc' } : {}),
          }}
          placeholder="Describe the changes you want to make across your codebase..."
          value={task}
          onChange={(e) => setTask(e.target.value)}
          onFocus={() => setInputFocused(true)}
          onBlur={() => setInputFocused(false)}
          disabled={status === 'composing' || status === 'applying'}
        />
        <div style={styles.submitRow}>
          <span style={styles.hint}>
            The AI will analyze your codebase and propose file-level changes.
          </span>
          {status === 'idle' && (
            <button
              style={{
                ...styles.submitButton,
                opacity: task.trim() ? 1 : 0.5,
                cursor: task.trim() ? 'pointer' : 'default',
              }}
              onClick={handleSubmit}
              disabled={!task.trim()}
              onMouseEnter={(e) => {
                if (task.trim()) (e.target as HTMLElement).style.background = '#1a8ad4';
              }}
              onMouseLeave={(e) => {
                (e.target as HTMLElement).style.background = '#007acc';
              }}
            >
              Generate Changes
            </button>
          )}
          {(status === 'done' || status === 'error') && (
            <button style={styles.cancelButton} onClick={handleReset}>
              New Task
            </button>
          )}
        </div>
      </div>

      {/* Loading state */}
      {status === 'composing' && fileChanges.length === 0 && (
        <div style={styles.loadingArea}>
          <div style={styles.spinner} />
          <span>Analyzing codebase and generating changes...</span>
        </div>
      )}

      {/* Error */}
      {status === 'error' && (
        <div style={styles.errorText}>{error}</div>
      )}

      {/* Changes summary */}
      {fileChanges.length > 0 && (
        <>
          <div style={styles.summary}>
            <span>{fileChanges.length} file{fileChanges.length !== 1 ? 's' : ''} changed</span>
            {acceptedCount > 0 && (
              <span style={{ color: '#4ec9b0' }}>{acceptedCount} accepted</span>
            )}
            {rejectedCount > 0 && (
              <span style={{ color: '#f44747' }}>{rejectedCount} rejected</span>
            )}
            {pendingCount > 0 && (
              <span style={{ color: '#dcdcaa' }}>{pendingCount} pending</span>
            )}
          </div>

          <div style={styles.changesArea}>
            <FileChanges
              changes={fileChanges}
              onAccept={handleAcceptFile}
              onReject={handleRejectFile}
            />
          </div>

          {status === 'reviewing' && (
            <div style={styles.actionsBar}>
              <button
                style={styles.rejectAllButton}
                onClick={handleRejectAll}
                onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'rgba(244, 71, 71, 0.1)'; }}
                onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
              >
                Reject All
              </button>
              <button
                style={styles.acceptAllButton}
                onClick={handleAcceptAll}
                onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#3dd1a6'; }}
                onMouseLeave={(e) => { (e.target as HTMLElement).style.background = '#4ec9b0'; }}
              >
                Accept All
              </button>
              {acceptedCount > 0 && (
                <button
                  style={styles.submitButton}
                  onClick={handleApply}
                  onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#1a8ad4'; }}
                  onMouseLeave={(e) => { (e.target as HTMLElement).style.background = '#007acc'; }}
                >
                  Apply {acceptedCount} Change{acceptedCount !== 1 ? 's' : ''}
                </button>
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}
