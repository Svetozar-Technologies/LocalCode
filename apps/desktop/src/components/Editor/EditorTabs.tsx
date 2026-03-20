import { useState, useCallback, useEffect, useRef } from 'react';
import type { OpenFile } from '../../types';
import { useAppStore } from '../../stores/appStore';

export default function EditorTabs() {
  const { openFiles, activeFile, setActiveFile, closeFile } = useAppStore();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; path: string } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const handleContextMenu = useCallback((e: React.MouseEvent, path: string) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, path });
  }, []);

  const handleSplitRight = useCallback(() => {
    if (!contextMenu) return;
    const store = useAppStore.getState();
    if (store.activeFile && store.activeFile !== contextMenu.path) {
      store.setSplitEditorRightPath(contextMenu.path);
    } else {
      // Split with same file
      store.setSplitEditorRightPath(contextMenu.path);
    }
    store.setSplitEditorMode('horizontal');
    setContextMenu(null);
  }, [contextMenu]);

  const handleCloseSplit = useCallback(() => {
    const store = useAppStore.getState();
    store.setSplitEditorMode('off');
    store.setSplitEditorRightPath(null);
    setContextMenu(null);
  }, []);

  // Feature 17: Open file history
  const handleFileHistory = useCallback(() => {
    if (!contextMenu) return;
    const store = useAppStore.getState();
    store.setSidebarView('git');
    // Store the file path to filter history — use a simple approach via URL hash
    (window as any).__fileHistoryFilter = contextMenu.path;
    setContextMenu(null);
  }, [contextMenu]);

  // Close context menu on click outside
  useEffect(() => {
    if (!contextMenu) return;
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [contextMenu]);

  if (openFiles.length === 0) return null;

  const splitMode = useAppStore.getState().splitEditorMode;

  return (
    <div className="editor-tabs">
      {openFiles.map((file: OpenFile) => (
        <div
          key={file.path}
          className={`editor-tab ${activeFile === file.path ? 'active' : ''}`}
          onClick={() => setActiveFile(file.path)}
          onContextMenu={(e) => handleContextMenu(e, file.path)}
        >
          {file.modified && <span className="tab-modified" />}
          <span className="tab-name">{file.name}</span>
          <span
            className="tab-close"
            onClick={(e) => {
              e.stopPropagation();
              closeFile(file.path);
            }}
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 8.707l3.646 3.647.708-.707L8.707 8l3.647-3.646-.707-.708L8 7.293 4.354 3.646l-.707.708L7.293 8l-3.646 3.646.707.708L8 8.707z" />
            </svg>
          </span>
        </div>
      ))}
      {contextMenu && (
        <div
          ref={menuRef}
          style={{
            position: 'fixed',
            left: contextMenu.x,
            top: contextMenu.y,
            background: 'var(--bg-secondary)',
            border: '1px solid var(--border-color)',
            borderRadius: 4,
            boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
            padding: '4px 0',
            zIndex: 1000,
            minWidth: 160,
            fontSize: 12,
            color: 'var(--text-primary)',
          }}
        >
          {splitMode === 'off' ? (
            <div
              style={{ padding: '6px 12px', cursor: 'pointer' }}
              onClick={handleSplitRight}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'transparent'; }}
            >
              Split Right
            </div>
          ) : (
            <div
              style={{ padding: '6px 12px', cursor: 'pointer' }}
              onClick={handleCloseSplit}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'transparent'; }}
            >
              Close Split
            </div>
          )}
          <div
            style={{ padding: '6px 12px', cursor: 'pointer' }}
            onClick={handleFileHistory}
            onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'transparent'; }}
          >
            File History
          </div>
          <div
            style={{ padding: '6px 12px', cursor: 'pointer' }}
            onClick={() => {
              closeFile(contextMenu.path);
              setContextMenu(null);
            }}
            onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'transparent'; }}
          >
            Close
          </div>
        </div>
      )}
    </div>
  );
}
