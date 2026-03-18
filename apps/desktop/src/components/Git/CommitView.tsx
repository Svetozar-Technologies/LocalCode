import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';

interface CommitViewProps {
  onRefresh: () => void;
}

const styles = {
  container: {
    padding: '10px 12px',
    borderBottom: '1px solid #3c3c3c',
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 8,
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  input: {
    width: '100%',
    background: '#3c3c3c',
    border: '1px solid #3c3c3c',
    borderRadius: 4,
    color: '#cccccc',
    padding: '8px 10px',
    fontSize: 12,
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    outline: 'none',
    resize: 'vertical' as const,
    minHeight: 60,
    maxHeight: 150,
    lineHeight: 1.5,
  } as React.CSSProperties,
  inputFocused: {
    borderColor: '#007acc',
  },
  actions: {
    display: 'flex',
    gap: 6,
    alignItems: 'center',
  } as React.CSSProperties,
  commitButton: {
    background: '#007acc',
    border: 'none',
    borderRadius: 4,
    color: '#ffffff',
    padding: '6px 16px',
    cursor: 'pointer',
    fontSize: 12,
    fontWeight: 500,
    flex: 1,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 6,
  } as React.CSSProperties,
  commitButtonDisabled: {
    opacity: 0.5,
    cursor: 'default',
  } as React.CSSProperties,
  secondaryButton: {
    background: 'none',
    border: '1px solid #3c3c3c',
    borderRadius: 4,
    color: '#cccccc',
    padding: '6px 12px',
    cursor: 'pointer',
    fontSize: 12,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 4,
  } as React.CSSProperties,
  dropdown: {
    position: 'relative' as const,
  } as React.CSSProperties,
  dropdownMenu: {
    position: 'absolute' as const,
    top: '100%',
    right: 0,
    marginTop: 4,
    background: '#252526',
    border: '1px solid #3c3c3c',
    borderRadius: 4,
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
    zIndex: 10,
    minWidth: 180,
    overflow: 'hidden',
  } as React.CSSProperties,
  dropdownItem: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '8px 12px',
    cursor: 'pointer',
    fontSize: 12,
    color: '#cccccc',
    transition: 'background 0.05s',
  } as React.CSSProperties,
  dropdownItemHover: {
    background: '#2a2d2e',
  } as React.CSSProperties,
  errorText: {
    color: '#f44747',
    fontSize: 11,
    padding: '4px 0',
  } as React.CSSProperties,
  successText: {
    color: '#4ec9b0',
    fontSize: 11,
    padding: '4px 0',
  } as React.CSSProperties,
  charCount: {
    fontSize: 10,
    color: '#6a6a6a',
    textAlign: 'right' as const,
  } as React.CSSProperties,
};

export default function CommitView({ onRefresh }: CommitViewProps) {
  const { projectPath, gitStatus } = useAppStore();
  const [message, setMessage] = useState('');
  const [inputFocused, setInputFocused] = useState(false);
  const [committing, setCommitting] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [showDropdown, setShowDropdown] = useState(false);

  const hasChanges = gitStatus.length > 0;
  const canCommit = message.trim().length > 0 && hasChanges && !committing;

  // Split message into subject and body
  const messageLines = message.split('\n');
  const subjectLength = messageLines[0]?.length || 0;
  const subjectTooLong = subjectLength > 72;

  const handleCommit = useCallback(async () => {
    if (!canCommit || !projectPath) return;

    setCommitting(true);
    setError('');
    setSuccess('');

    try {
      await invoke('git_commit', {
        path: projectPath,
        message: message.trim(),
      });
      setSuccess('Changes committed successfully.');
      setMessage('');
      onRefresh();

      // Clear success message after a delay
      setTimeout(() => setSuccess(''), 3000);
    } catch (err) {
      setError(`Commit failed: ${err}`);
    }

    setCommitting(false);
  }, [canCommit, projectPath, message, onRefresh]);

  const handleCommitAll = useCallback(async () => {
    if (!message.trim() || !projectPath) return;

    setCommitting(true);
    setError('');
    setSuccess('');
    setShowDropdown(false);

    try {
      await invoke('git_add_all', { path: projectPath });
      await invoke('git_commit', {
        path: projectPath,
        message: message.trim(),
      });
      setSuccess('All changes committed successfully.');
      setMessage('');
      onRefresh();
      setTimeout(() => setSuccess(''), 3000);
    } catch (err) {
      setError(`Commit failed: ${err}`);
    }

    setCommitting(false);
  }, [message, projectPath, onRefresh]);

  const handleAmend = useCallback(async () => {
    if (!projectPath) return;

    setCommitting(true);
    setError('');
    setSuccess('');
    setShowDropdown(false);

    try {
      await invoke('git_commit_amend', {
        path: projectPath,
        message: message.trim() || undefined,
      });
      setSuccess('Commit amended successfully.');
      setMessage('');
      onRefresh();
      setTimeout(() => setSuccess(''), 3000);
    } catch (err) {
      setError(`Amend failed: ${err}`);
    }

    setCommitting(false);
  }, [projectPath, message, onRefresh]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleCommit();
    }
  };

  return (
    <div style={styles.container}>
      <textarea
        style={{
          ...styles.input,
          ...(inputFocused ? { borderColor: '#007acc' } : {}),
        }}
        placeholder="Commit message (Cmd+Enter to commit)"
        value={message}
        onChange={(e) => {
          setMessage(e.target.value);
          setError('');
        }}
        onFocus={() => setInputFocused(true)}
        onBlur={() => setInputFocused(false)}
        onKeyDown={handleKeyDown}
        disabled={committing}
      />

      {message.length > 0 && (
        <div style={{
          ...styles.charCount,
          color: subjectTooLong ? '#f44747' : '#6a6a6a',
        }}>
          {subjectLength}/72 characters {subjectTooLong ? '(too long)' : ''}
        </div>
      )}

      {error && <div style={styles.errorText}>{error}</div>}
      {success && <div style={styles.successText}>{success}</div>}

      <div style={styles.actions}>
        <button
          style={{
            ...styles.commitButton,
            ...(!canCommit ? styles.commitButtonDisabled : {}),
          }}
          onClick={handleCommit}
          disabled={!canCommit}
          onMouseEnter={(e) => {
            if (canCommit) (e.target as HTMLElement).style.background = '#1a8ad4';
          }}
          onMouseLeave={(e) => {
            (e.target as HTMLElement).style.background = '#007acc';
          }}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z" />
          </svg>
          {committing ? 'Committing...' : 'Commit'}
        </button>

        <div style={styles.dropdown}>
          <button
            style={styles.secondaryButton}
            onClick={() => setShowDropdown(!showDropdown)}
            title="More commit options"
            onMouseEnter={(e) => { (e.target as HTMLElement).style.borderColor = '#969696'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.borderColor = '#3c3c3c'; }}
          >
            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 6l4 4 4-4" stroke="currentColor" strokeWidth="1.5" fill="none" />
            </svg>
          </button>

          {showDropdown && (
            <div style={styles.dropdownMenu}>
              <div
                style={styles.dropdownItem}
                onClick={handleCommitAll}
                onMouseEnter={(e) => {
                  Object.assign((e.currentTarget as HTMLElement).style, styles.dropdownItemHover);
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.background = 'transparent';
                }}
              >
                <svg width="14" height="14" viewBox="0 0 16 16" fill="#4ec9b0">
                  <path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z" />
                </svg>
                Commit All
              </div>
              <div
                style={styles.dropdownItem}
                onClick={handleAmend}
                onMouseEnter={(e) => {
                  Object.assign((e.currentTarget as HTMLElement).style, styles.dropdownItemHover);
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.background = 'transparent';
                }}
              >
                <svg width="14" height="14" viewBox="0 0 16 16" fill="#dcdcaa">
                  <path d="M13.23 1h-1.46L3.52 9.25l-.16.22L1 13.59 2.41 15l4.12-2.36.22-.16L15 4.23V2.77L13.23 1zM2.41 13.59l1.51-3 1.45 1.45-2.96 1.55zm3.83-2.06L4.47 9.76l8-8 1.77 1.77-8 8z" />
                </svg>
                Amend Last Commit
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
