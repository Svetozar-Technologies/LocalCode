import React, { useCallback, useEffect } from 'react';
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
import QuickOpen from './components/Editor/QuickOpen';
import CommandPalette from './components/Editor/CommandPalette';
import Composer from './components/Composer/Composer';
import SetupWizard from './components/Settings/SetupWizard';
import { useAppStore } from './stores/appStore';

function Sidebar() {
  const { sidebarView, sidebarWidth, chatPanelVisible } = useAppStore();

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
        chatPanelVisible ? (
          <div className="sidebar" style={{ padding: 16, color: 'var(--text-secondary)', fontSize: 13 }}>
            <div className="sidebar-header">AI Assistant</div>
            <p style={{ margin: '12px 0' }}>AI Chat is open in the right panel.</p>
            <button
              className="action-btn"
              style={{ fontSize: 12, padding: '4px 10px' }}
              onClick={() => useAppStore.getState().toggleChatPanel()}
            >
              Move here
            </button>
          </div>
        ) : (
          <>
            <div className="sidebar-header">AI Assistant</div>
            <ChatPanel />
          </>
        )
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
      {sidebarView === 'composer' && (
        <>
          <div className="sidebar-header">Composer</div>
          <Composer />
        </>
      )}
    </div>
  );
}

function RightPanel() {
  const { chatPanelWidth } = useAppStore();

  return (
    <div className="right-panel" style={{ width: chatPanelWidth, minWidth: 280, maxWidth: 600 }}>
      <div className="sidebar-header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span>AI Chat</span>
        <button
          className="action-btn"
          onClick={() => useAppStore.getState().toggleChatPanel()}
          title="Close AI Chat (Cmd+I)"
          style={{ padding: '2px 6px', fontSize: 11 }}
        >
          ✕
        </button>
      </div>
      <ChatPanel />
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

// --- Error Boundary ---
class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { hasError: boolean; error: Error | null }
> {
  state = { hasError: false, error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  render() {
    if (this.state.hasError) {
      return (
        <div style={{
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
          height: '100vh', background: 'var(--bg-primary)', color: 'var(--text-primary)',
          fontFamily: 'var(--font-ui)', padding: 32, textAlign: 'center',
        }}>
          <h2 style={{ marginBottom: 12 }}>Something went wrong</h2>
          <p style={{ color: 'var(--text-secondary)', marginBottom: 16, maxWidth: 480 }}>
            {this.state.error?.message || 'An unexpected error occurred.'}
          </p>
          <button
            onClick={() => {
              this.setState({ hasError: false, error: null });
              window.location.reload();
            }}
            style={{
              padding: '8px 20px', borderRadius: 6, border: 'none',
              background: 'var(--accent)', color: '#fff', cursor: 'pointer', fontSize: 13,
            }}
          >
            Reload App
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

// --- Toast Container ---
function ToastContainer() {
  const toasts = useAppStore((s) => s.toasts);
  const removeToast = useAppStore((s) => s.removeToast);

  if (toasts.length === 0) return null;

  return (
    <div style={{
      position: 'fixed', bottom: 32, right: 32, zIndex: 10001,
      display: 'flex', flexDirection: 'column', gap: 8,
    }}>
      {toasts.map((toast) => (
        <div
          key={toast.id}
          onClick={() => removeToast(toast.id)}
          style={{
            padding: '10px 16px', borderRadius: 8, fontSize: 13, cursor: 'pointer',
            maxWidth: 360, boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
            background: toast.type === 'error' ? '#c53030' : toast.type === 'success' ? '#2f855a' : 'var(--bg-tertiary)',
            color: '#fff', border: '1px solid rgba(255,255,255,0.1)',
            animation: 'fadeIn 0.2s ease-out',
          }}
        >
          {toast.message}
        </div>
      ))}
    </div>
  );
}

function App() {
  const { terminalVisible, sidebarWidth, setSidebarWidth, terminalHeight, setTerminalHeight, toggleTerminal, chatPanelVisible, chatPanelWidth, setChatPanelWidth } = useAppStore();
  const setupComplete = useAppStore((s) => s.setupComplete);
  const setSetupComplete = useAppStore((s) => s.setSetupComplete);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        if (e.key === '`') {
          e.preventDefault();
          toggleTerminal();
        }
        if (e.key === 'i' && !e.shiftKey) {
          e.preventDefault();
          useAppStore.getState().toggleChatPanel();
        }
        if (e.key === 'b') {
          e.preventDefault();
          const store = useAppStore.getState();
          store.setSidebarWidth(store.sidebarWidth > 0 ? 0 : 260);
        }
        if (e.key === 'p' && e.shiftKey) {
          e.preventDefault();
          useAppStore.getState().toggleCommandPalette();
          return;
        }
        if (e.key === 'p') {
          e.preventDefault();
          useAppStore.getState().toggleQuickOpen();
        }
        if (e.key === 'k') {
          e.preventDefault();
          useAppStore.getState().setInlineEditVisible(true);
        }
        if (e.key === 'f' && !e.shiftKey) {
          e.preventDefault();
          useAppStore.getState().toggleFindReplace();
        }
        if (e.key === '\\') {
          e.preventDefault();
          const store = useAppStore.getState();
          if (store.splitEditorMode !== 'off') {
            store.setSplitEditorMode('off');
            store.setSplitEditorRightPath(null);
          } else if (store.activeFile) {
            // Pick the next open file as the right pane, or same file
            const otherFile = store.openFiles.find((f) => f.path !== store.activeFile);
            store.setSplitEditorRightPath(otherFile ? otherFile.path : store.activeFile);
            store.setSplitEditorMode('horizontal');
          }
        }
        // Cmd+Shift+I for Composer
        if (e.key === 'i' && e.shiftKey) {
          e.preventDefault();
          const store = useAppStore.getState();
          store.setSidebarView('composer');
          if (store.sidebarWidth === 0) store.setSidebarWidth(260);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleTerminal]);

  const quickOpenVisible = useAppStore((s) => s.quickOpenVisible);
  const commandPaletteVisible = useAppStore((s) => s.commandPaletteVisible);

  return (
    <div className="app-container">
      {!setupComplete && (
        <SetupWizard onComplete={() => setSetupComplete(true)} />
      )}
      <ToastContainer />
      <QuickOpen
        visible={quickOpenVisible}
        onClose={() => useAppStore.getState().setQuickOpenVisible(false)}
      />
      <CommandPalette
        visible={commandPaletteVisible}
        onClose={() => useAppStore.getState().toggleCommandPalette()}
      />
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
        {chatPanelVisible && (
          <>
            <ResizeHandle
              direction="horizontal"
              onResize={(delta) => {
                setChatPanelWidth(Math.max(280, Math.min(600, chatPanelWidth + delta)));
              }}
            />
            <RightPanel />
          </>
        )}
      </div>
      <StatusBar />
    </div>
  );
}

function AppWithErrorBoundary() {
  return (
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  );
}

export default AppWithErrorBoundary;
