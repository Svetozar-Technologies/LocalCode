import { useState, useRef, useCallback } from 'react';
import { useAppStore } from '../../stores/appStore';

export default function AppPreview() {
  const { appPreviewUrl, setAppPreviewUrl, toggleAppPreview } = useAppStore();
  const [urlInput, setUrlInput] = useState(appPreviewUrl);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  const handleNavigate = useCallback(() => {
    let url = urlInput.trim();
    if (url && !url.startsWith('http://') && !url.startsWith('https://')) {
      url = 'http://' + url;
    }
    setAppPreviewUrl(url);
  }, [urlInput, setAppPreviewUrl]);

  const handleRefresh = useCallback(() => {
    if (iframeRef.current) {
      iframeRef.current.src = appPreviewUrl;
    }
  }, [appPreviewUrl]);

  return (
    <div className="app-preview-panel">
      <div className="app-preview-toolbar">
        <button
          onClick={handleRefresh}
          style={{
            background: 'none',
            border: 'none',
            color: 'var(--text-secondary)',
            cursor: 'pointer',
            padding: '2px 4px',
          }}
          title="Refresh"
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M13.451 5.609l-.579-.101.076-.464a6 6 0 10-3.84 7.963l.3.95A7 7 0 1114.5 5.5l-.579.101z" />
          </svg>
        </button>
        <input
          value={urlInput}
          onChange={(e) => setUrlInput(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') handleNavigate(); }}
          placeholder="http://localhost:3000"
        />
        <button
          onClick={handleNavigate}
          style={{
            background: 'var(--accent)',
            border: 'none',
            borderRadius: 3,
            color: '#fff',
            padding: '2px 8px',
            cursor: 'pointer',
            fontSize: 11,
          }}
        >
          Go
        </button>
        <button
          onClick={toggleAppPreview}
          style={{
            background: 'none',
            border: 'none',
            color: 'var(--text-muted)',
            cursor: 'pointer',
            padding: '2px 4px',
          }}
          title="Close Preview"
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 8.707l3.646 3.647.708-.707L8.707 8l3.647-3.646-.707-.708L8 7.293 4.354 3.646l-.707.708L7.293 8l-3.646 3.646.707.708L8 8.707z" />
          </svg>
        </button>
      </div>
      <iframe
        ref={iframeRef}
        className="app-preview-iframe"
        src={appPreviewUrl}
        title="App Preview"
        sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
      />
    </div>
  );
}
