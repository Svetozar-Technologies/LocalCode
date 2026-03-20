import { useState, useRef, useEffect, useCallback } from 'react';
import type { editor } from 'monaco-editor';

interface IFindController extends editor.IEditorContribution {
  start(opts: {
    searchString: string;
    isRegex: boolean;
    matchCase: boolean;
    wholeWord: boolean;
    replaceString: string;
  }): void;
  moveToNextMatch(): void;
  moveToPreviousMatch(): void;
}

interface FindReplaceProps {
  editorInstance: editor.IStandaloneCodeEditor | null;
  visible: boolean;
  onClose: () => void;
}

const styles = {
  overlay: {
    position: 'absolute' as const,
    top: 0,
    right: 20,
    zIndex: 50,
    background: 'var(--bg-secondary)',
    border: '1px solid #3c3c3c',
    borderTop: 'none',
    borderRadius: '0 0 4px 4px',
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
    padding: '8px 12px',
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 6,
    minWidth: 380,
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  row: {
    display: 'flex',
    alignItems: 'center',
    gap: 4,
  } as React.CSSProperties,
  input: {
    flex: 1,
    background: 'var(--border-color)',
    border: '1px solid #3c3c3c',
    borderRadius: 3,
    color: 'var(--text-primary)',
    padding: '4px 8px',
    fontSize: 13,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    outline: 'none',
    minWidth: 0,
  } as React.CSSProperties,
  inputFocused: {
    borderColor: '#007acc',
  } as React.CSSProperties,
  toggleButton: {
    width: 26,
    height: 26,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'none',
    border: '1px solid transparent',
    borderRadius: 3,
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    fontSize: 12,
    fontWeight: 600,
    flexShrink: 0,
  } as React.CSSProperties,
  toggleButtonActive: {
    background: '#264f78',
    borderColor: '#007acc',
    color: 'var(--text-primary)',
  } as React.CSSProperties,
  actionButton: {
    width: 26,
    height: 26,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'none',
    border: 'none',
    borderRadius: 3,
    color: 'var(--text-primary)',
    cursor: 'pointer',
    flexShrink: 0,
  } as React.CSSProperties,
  matchInfo: {
    fontSize: 11,
    color: 'var(--text-secondary)',
    padding: '0 6px',
    whiteSpace: 'nowrap' as const,
    flexShrink: 0,
    minWidth: 60,
    textAlign: 'right' as const,
  } as React.CSSProperties,
  closeButton: {
    width: 24,
    height: 24,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'none',
    border: 'none',
    borderRadius: 3,
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    marginLeft: 4,
    flexShrink: 0,
  } as React.CSSProperties,
  expandButton: {
    width: 20,
    height: 26,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'none',
    border: 'none',
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    flexShrink: 0,
  } as React.CSSProperties,
};

export default function FindReplace({ editorInstance, visible, onClose }: FindReplaceProps) {
  const [searchText, setSearchText] = useState('');
  const [replaceText, setReplaceText] = useState('');
  const [matchCase, setMatchCase] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [useRegex, setUseRegex] = useState(false);
  const [showReplace, setShowReplace] = useState(false);
  const [matchCount, setMatchCount] = useState(0);
  const [currentMatch, setCurrentMatch] = useState(0);
  const [searchFocused, setSearchFocused] = useState(false);
  const [replaceFocused, setReplaceFocused] = useState(false);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const findControllerRef = useRef<IFindController | null>(null);

  // Focus search input when visible
  useEffect(() => {
    if (visible) {
      setTimeout(() => searchInputRef.current?.focus(), 50);
    }
  }, [visible]);

  // Trigger search through Monaco's find controller
  const triggerFind = useCallback(() => {
    if (!editorInstance || !searchText) {
      setMatchCount(0);
      setCurrentMatch(0);
      return;
    }

    const model = editorInstance.getModel();
    if (!model) return;

    // Use Monaco's built-in find functionality
    const findController = editorInstance.getContribution<IFindController>('editor.contrib.findController');
    if (findController) {
      findControllerRef.current = findController;

      // Start find with options
      findController.start({
        searchString: searchText,
        isRegex: useRegex,
        matchCase,
        wholeWord,
        replaceString: replaceText,
      });
    }

    // Count matches manually for display
    try {
      let searchPattern: string | RegExp;
      if (useRegex) {
        searchPattern = new RegExp(searchText, matchCase ? 'g' : 'gi');
      } else {
        const escaped = searchText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        const pattern = wholeWord ? `\\b${escaped}\\b` : escaped;
        searchPattern = new RegExp(pattern, matchCase ? 'g' : 'gi');
      }

      const text = model.getValue();
      const matches = text.match(searchPattern);
      setMatchCount(matches ? matches.length : 0);
      if (matches && matches.length > 0 && currentMatch === 0) {
        setCurrentMatch(1);
      }
    } catch {
      // Invalid regex
      setMatchCount(0);
      setCurrentMatch(0);
    }
  }, [editorInstance, searchText, matchCase, wholeWord, useRegex, replaceText, currentMatch]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    triggerFind();
  }, [triggerFind]);

  const findNext = useCallback(() => {
    if (!editorInstance) return;
    const findController = editorInstance.getContribution<IFindController>('editor.contrib.findController');
    if (findController) {
      findController.moveToNextMatch();
      setCurrentMatch((prev) => (prev >= matchCount ? 1 : prev + 1));
    }
  }, [editorInstance, matchCount]);

  const findPrevious = useCallback(() => {
    if (!editorInstance) return;
    const findController = editorInstance.getContribution<IFindController>('editor.contrib.findController');
    if (findController) {
      findController.moveToPreviousMatch();
      setCurrentMatch((prev) => (prev <= 1 ? matchCount : prev - 1));
    }
  }, [editorInstance, matchCount]);

  const replaceCurrent = useCallback(() => {
    if (!editorInstance || !searchText) return;

    const selection = editorInstance.getSelection();
    if (!selection) return;

    const model = editorInstance.getModel();
    if (!model) return;

    const selectedText = model.getValueInRange(selection);
    const isMatch = matchCase
      ? selectedText === searchText
      : selectedText.toLowerCase() === searchText.toLowerCase();

    if (isMatch) {
      editorInstance.executeEdits('find-replace', [
        { range: selection, text: replaceText },
      ]);
      findNext();
      setMatchCount((prev) => Math.max(0, prev - 1));
    } else {
      findNext();
    }
  }, [editorInstance, searchText, replaceText, matchCase, findNext]);

  const replaceAll = useCallback(() => {
    if (!editorInstance || !searchText) return;

    const model = editorInstance.getModel();
    if (!model) return;

    const text = model.getValue();
    let newText: string;

    try {
      if (useRegex) {
        const regex = new RegExp(searchText, matchCase ? 'g' : 'gi');
        newText = text.replace(regex, replaceText);
      } else {
        const escaped = searchText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        const pattern = wholeWord ? `\\b${escaped}\\b` : escaped;
        const regex = new RegExp(pattern, matchCase ? 'g' : 'gi');
        newText = text.replace(regex, replaceText);
      }

      const fullRange = model.getFullModelRange();
      editorInstance.executeEdits('find-replace-all', [
        { range: fullRange, text: newText },
      ]);

      setMatchCount(0);
      setCurrentMatch(0);
    } catch {
      // Invalid regex
    }
  }, [editorInstance, searchText, replaceText, matchCase, wholeWord, useRegex]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      } else if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        findNext();
      } else if (e.key === 'Enter' && e.shiftKey) {
        e.preventDefault();
        findPrevious();
      }
    },
    [onClose, findNext, findPrevious]
  );

  if (!visible) return null;

  return (
    <div style={styles.overlay}>
      {/* Find row */}
      <div style={styles.row}>
        <button
          style={styles.expandButton}
          onClick={() => setShowReplace(!showReplace)}
          title={showReplace ? 'Hide Replace' : 'Show Replace'}
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 16 16"
            fill="currentColor"
            style={{
              transform: showReplace ? 'rotate(90deg)' : 'rotate(0deg)',
              transition: 'transform 0.15s',
            }}
          >
            <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" fill="none" />
          </svg>
        </button>
        <input
          ref={searchInputRef}
          type="text"
          style={{
            ...styles.input,
            ...(searchFocused ? styles.inputFocused : {}),
          }}
          placeholder="Find"
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          onKeyDown={handleKeyDown}
          onFocus={() => setSearchFocused(true)}
          onBlur={() => setSearchFocused(false)}
        />
        <button
          style={{
            ...styles.toggleButton,
            ...(matchCase ? styles.toggleButtonActive : {}),
          }}
          onClick={() => setMatchCase(!matchCase)}
          title="Match Case (Alt+C)"
        >
          Aa
        </button>
        <button
          style={{
            ...styles.toggleButton,
            ...(wholeWord ? styles.toggleButtonActive : {}),
          }}
          onClick={() => setWholeWord(!wholeWord)}
          title="Match Whole Word (Alt+W)"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M1 12h2v1H0v-1.5l3-4.5H0V6h3.5v1.5L1 12zm5-6h3.5L7 12h1.5l.7-2h2.6l.7 2H14L11.5 6H9zm1.2 3l.8-2.4.8 2.4H7.2z" />
          </svg>
        </button>
        <button
          style={{
            ...styles.toggleButton,
            ...(useRegex ? styles.toggleButtonActive : {}),
          }}
          onClick={() => setUseRegex(!useRegex)}
          title="Use Regular Expression (Alt+R)"
        >
          .*
        </button>
        <span style={styles.matchInfo}>
          {searchText ? `${currentMatch} of ${matchCount}` : 'No results'}
        </span>
        <button
          style={styles.actionButton}
          onClick={findPrevious}
          title="Previous Match (Shift+Enter)"
          onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
          onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 8l4-4 .7.7L5.4 8l3.3 3.3-.7.7L4 8z" />
          </svg>
        </button>
        <button
          style={styles.actionButton}
          onClick={findNext}
          title="Next Match (Enter)"
          onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
          onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M12 8l-4 4-.7-.7L10.6 8 7.3 4.7l.7-.7L12 8z" />
          </svg>
        </button>
        <button
          style={styles.closeButton}
          onClick={onClose}
          title="Close (Escape)"
          onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
          onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 8.707l3.646 3.647.708-.707L8.707 8l3.647-3.646-.707-.708L8 7.293 4.354 3.646l-.707.708L7.293 8l-3.646 3.646.707.708L8 8.707z" />
          </svg>
        </button>
      </div>

      {/* Replace row */}
      {showReplace && (
        <div style={{ ...styles.row, paddingLeft: 20 }}>
          <input
            type="text"
            style={{
              ...styles.input,
              ...(replaceFocused ? styles.inputFocused : {}),
            }}
            placeholder="Replace"
            value={replaceText}
            onChange={(e) => setReplaceText(e.target.value)}
            onKeyDown={handleKeyDown}
            onFocus={() => setReplaceFocused(true)}
            onBlur={() => setReplaceFocused(false)}
          />
          <button
            style={styles.actionButton}
            onClick={replaceCurrent}
            title="Replace (Ctrl+Shift+1)"
            onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M3.221 3.739l2.261 2.269L7.7 3.784l-.7-.7-1.012 1.007-.008-1.6a3 3 0 013.72 2.9l1-.1a4 4 0 00-4.72-3.8l.008 1.6-.993-1.007-.694.655zM12.779 12.261l-2.261-2.269L8.3 12.216l.7.7 1.012-1.007.008 1.6a3 3 0 01-3.72-2.9l-1 .1a4 4 0 004.72 3.8l-.008-1.6.993 1.007.694-.655z" />
            </svg>
          </button>
          <button
            style={styles.actionButton}
            onClick={replaceAll}
            title="Replace All (Ctrl+Alt+Enter)"
            onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M11.6 2.677c-.603-.282-1.313-.17-1.842.19l-.395.27.242.342.395-.27a1.224 1.224 0 011.413-.146c.468.22.748.702.712 1.222-.036.52-.373.948-.867 1.09l-.36.105.13.394.36-.105c.652-.19 1.11-.76 1.157-1.453a1.624 1.624 0 00-.945-1.639zM6.273 5.5h3.727V6.6H6.273V5.5zm0 2.2h3.727v1.1H6.273V7.7zm0 2.2h3.727v1.1H6.273V9.9zM3.727 5.5h1.364V6.6H3.727V5.5zm0 2.2h1.364v1.1H3.727V7.7zm0 2.2h1.364v1.1H3.727V9.9z" />
            </svg>
          </button>
        </div>
      )}
    </div>
  );
}
