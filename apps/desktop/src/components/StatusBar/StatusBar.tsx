import { useAppStore } from '../../stores/appStore';

export default function StatusBar() {
  const { activeFile, openFiles, currentBranch, llmConnected, llmConfig, agentMode, problems, setBottomPanelTab } = useAppStore();

  const currentFile = openFiles.find((f) => f.path === activeFile);
  const ext = activeFile?.split('.').pop() || '';

  const errorCount = problems.filter((p) => p.severity === 'error').length;
  const warningCount = problems.filter((p) => p.severity === 'warning').length;

  return (
    <div className="status-bar">
      <span className="status-item" title="Git Branch">
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
          <path d="M15 5c0-1.66-1.34-3-3-3S9 3.34 9 5c0 1.3.84 2.4 2 2.82V9c0 .55-.45 1-1 1H6c-.55 0-1-.45-1-1V7.82C6.16 7.4 7 6.3 7 5c0-1.66-1.34-3-3-3S1 3.34 1 5c0 1.3.84 2.4 2 2.82V9c0 1.1.9 2 2 2h4c1.1 0 2-.9 2-2V7.82C12.16 7.4 13 6.3 13 5z" />
        </svg>
        {currentBranch || 'main'}
      </span>
      {(errorCount > 0 || warningCount > 0) && (
        <span
          className="status-item status-problems-badge"
          onClick={() => {
            setBottomPanelTab('problems');
            useAppStore.getState().terminalVisible || useAppStore.getState().toggleTerminal();
          }}
          style={{ cursor: 'pointer' }}
          title="Click to open Problems"
        >
          {errorCount > 0 && (
            <span style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
                <circle cx="8" cy="8" r="6" fill="#f44747" />
                <text x="8" y="11" textAnchor="middle" fill="white" fontSize="9" fontWeight="bold">{'\u2716'}</text>
              </svg>
              {errorCount}
            </span>
          )}
          {warningCount > 0 && (
            <span style={{ display: 'flex', alignItems: 'center', gap: 2 }}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
                <path d="M7.56 1h.88L16 14H0L7.56 1z" fill="#dcdcaa" />
              </svg>
              {warningCount}
            </span>
          )}
        </span>
      )}
      <span className="status-item">
        {llmConnected ? (
          <>
            <span style={{ width: 6, height: 6, borderRadius: '50%', background: '#4ec9b0', display: 'inline-block' }} />
            {llmConfig.modelName}
          </>
        ) : (
          <>
            <span style={{ width: 6, height: 6, borderRadius: '50%', background: '#f44747', display: 'inline-block' }} />
            No Model
          </>
        )}
      </span>
      {agentMode && (
        <span className="status-item" style={{ background: 'rgba(255,255,255,0.15)', padding: '0 6px', borderRadius: 3 }}>
          Agent Mode
        </span>
      )}
      <div className="status-right">
        {currentFile && (
          <>
            <span className="status-item">{ext.toUpperCase()}</span>
            <span className="status-item">UTF-8</span>
            <span className="status-item">
              {currentFile.content.split('\n').length} lines
            </span>
          </>
        )}
        <span className="status-item">LocalCode v0.2.0</span>
      </div>
    </div>
  );
}
