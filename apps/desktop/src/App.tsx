import { useCallback, useEffect } from 'react';
import ActivityBar from './components/ActivityBar/ActivityBar';
import FileExplorer from './components/Sidebar/FileExplorer';
import SearchPanel from './components/Search/SearchPanel';
import ChatPanel from './components/AIChat/ChatPanel';
import EditorPanel from './components/Editor/EditorPanel';
import TerminalPanel from './components/Terminal/Terminal';
import StatusBar from './components/StatusBar/StatusBar';
import GitPanel from './components/Git/GitPanel';
import DebugPanel from './components/Debug/DebugPanel';
import SettingsPanel from './components/Settings/SettingsPanel';
import { useAppStore } from './stores/appStore';

function Sidebar() {
  const { sidebarView, sidebarWidth } = useAppStore();

  return (
    <div className="sidebar" style={{ width: sidebarWidth, minWidth: 180, maxWidth: 500 }}>
      {sidebarView === 'explorer' && (
        <>
          <div className="sidebar-header">Explorer</div>
          <FileExplorer />
        </>
      )}
      {sidebarView === 'search' && (
        <>
          <div className="sidebar-header">Search</div>
          <SearchPanel />
        </>
      )}
      {sidebarView === 'git' && (
        <>
          <div className="sidebar-header">Source Control</div>
          <GitPanel />
        </>
      )}
      {sidebarView === 'ai' && (
        <>
          <div className="sidebar-header">AI Assistant</div>
          <ChatPanel />
        </>
      )}
      {sidebarView === 'debug' && (
        <>
          <div className="sidebar-header">Debug</div>
          <DebugPanel />
        </>
      )}
      {sidebarView === 'settings' && (
        <>
          <div className="sidebar-header">Settings</div>
          <SettingsPanel />
        </>
      )}
    </div>
  );
}

function ResizeHandle({
  direction,
  onResize,
}: {
  direction: 'horizontal' | 'vertical';
  onResize: (delta: number) => void;
}) {
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      const startPos = direction === 'horizontal' ? e.clientX : e.clientY;

      const handleMouseMove = (e: MouseEvent) => {
        const currentPos = direction === 'horizontal' ? e.clientX : e.clientY;
        onResize(currentPos - startPos);
      };

      const handleMouseUp = () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = direction === 'horizontal' ? 'col-resize' : 'row-resize';
      document.body.style.userSelect = 'none';
    },
    [direction, onResize]
  );

  return (
    <div
      className="resize-handle"
      onMouseDown={handleMouseDown}
      style={{
        ...(direction === 'horizontal'
          ? { width: 4, cursor: 'col-resize', minWidth: 4 }
          : { height: 4, cursor: 'row-resize', minHeight: 4 }),
      }}
    />
  );
}

function App() {
  const { terminalVisible, sidebarWidth, setSidebarWidth, terminalHeight, setTerminalHeight, toggleTerminal } = useAppStore();

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        if (e.key === '`') {
          e.preventDefault();
          toggleTerminal();
        }
        if (e.key === 'i') {
          e.preventDefault();
          useAppStore.getState().setSidebarView('ai');
        }
        if (e.key === 'b') {
          e.preventDefault();
          const store = useAppStore.getState();
          store.setSidebarWidth(store.sidebarWidth > 0 ? 0 : 260);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleTerminal]);

  return (
    <div className="app-container">
      <div className="app-main">
        <ActivityBar />
        {sidebarWidth > 0 && (
          <>
            <Sidebar />
            <ResizeHandle
              direction="horizontal"
              onResize={(delta) => {
                setSidebarWidth(Math.max(180, Math.min(500, sidebarWidth + delta)));
              }}
            />
          </>
        )}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <EditorPanel />
          {terminalVisible && (
            <>
              <ResizeHandle
                direction="vertical"
                onResize={(delta) => {
                  setTerminalHeight(Math.max(100, Math.min(600, terminalHeight - delta)));
                }}
              />
              <div style={{ height: terminalHeight }}>
                <TerminalPanel />
              </div>
            </>
          )}
        </div>
      </div>
      <StatusBar />
    </div>
  );
}

export default App;
