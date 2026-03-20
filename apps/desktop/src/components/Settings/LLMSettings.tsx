import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from '../../stores/appStore';

interface LLMProviderConfig {
  id: string;
  name: string;
  type: 'local' | 'openai' | 'anthropic' | 'openai-compatible';
  apiKey: string;
  baseUrl: string;
  model: string;
  enabled: boolean;
}

interface CatalogEntry {
  id: string;
  name: string;
  description: string;
  url: string;
  filename: string;
  size_bytes: number;
  quantization: string;
  context_length: number;
  parameters: string;
  family: string;
  tags: string[];
  downloaded: boolean;
}

interface DownloadedModel {
  catalog_id: string;
  path: string;
  name: string;
  size_bytes: number;
  downloaded_at: number;
}

type ConnectionStatus = 'idle' | 'testing' | 'connected' | 'failed';

const styles = {
  container: {
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  sectionTitle: {
    fontSize: 18,
    fontWeight: 600,
    color: 'var(--text-primary)',
    marginBottom: 20,
    paddingBottom: 8,
    borderBottom: '1px solid #3c3c3c',
  } as React.CSSProperties,
  subsectionTitle: {
    fontSize: 14,
    fontWeight: 600,
    color: 'var(--text-primary)',
    marginBottom: 12,
    marginTop: 24,
  } as React.CSSProperties,
  providerCard: {
    background: 'var(--bg-secondary)',
    border: '1px solid #3c3c3c',
    borderRadius: 6,
    marginBottom: 16,
    overflow: 'hidden',
  } as React.CSSProperties,
  providerCardActive: {
    borderColor: '#007acc',
  } as React.CSSProperties,
  providerHeader: {
    display: 'flex',
    alignItems: 'center',
    padding: '12px 16px',
    cursor: 'pointer',
    gap: 10,
    borderBottom: '1px solid transparent',
  } as React.CSSProperties,
  providerHeaderExpanded: {
    borderBottomColor: 'var(--border-color)',
  } as React.CSSProperties,
  providerName: {
    fontSize: 14,
    fontWeight: 500,
    color: 'var(--text-primary)',
    flex: 1,
  } as React.CSSProperties,
  providerType: {
    fontSize: 10,
    padding: '2px 8px',
    borderRadius: 8,
    fontWeight: 600,
    flexShrink: 0,
  } as React.CSSProperties,
  enableToggle: {
    position: 'relative' as const,
    width: 36,
    height: 20,
    borderRadius: 10,
    cursor: 'pointer',
    transition: 'background 0.2s',
    flexShrink: 0,
  } as React.CSSProperties,
  enableToggleKnob: {
    position: 'absolute' as const,
    top: 2,
    width: 16,
    height: 16,
    borderRadius: '50%',
    background: '#ffffff',
    transition: 'left 0.2s',
    boxShadow: '0 1px 3px rgba(0,0,0,0.3)',
  } as React.CSSProperties,
  providerBody: {
    padding: '16px',
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 14,
  } as React.CSSProperties,
  fieldGroup: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 4,
  } as React.CSSProperties,
  fieldLabel: {
    fontSize: 12,
    fontWeight: 500,
    color: 'var(--text-secondary)',
  } as React.CSSProperties,
  input: {
    width: '100%',
    maxWidth: 450,
    background: 'var(--border-color)',
    border: '1px solid #3c3c3c',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '7px 10px',
    fontSize: 13,
    outline: 'none',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  inputMono: {
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    fontSize: 12,
  } as React.CSSProperties,
  select: {
    maxWidth: 450,
    background: 'var(--border-color)',
    border: '1px solid #3c3c3c',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '7px 10px',
    fontSize: 13,
    outline: 'none',
  } as React.CSSProperties,
  actions: {
    display: 'flex',
    gap: 8,
    alignItems: 'center',
    marginTop: 4,
  } as React.CSSProperties,
  testButton: {
    background: 'none',
    border: '1px solid #3c3c3c',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '6px 16px',
    cursor: 'pointer',
    fontSize: 12,
    display: 'flex',
    alignItems: 'center',
    gap: 6,
  } as React.CSSProperties,
  saveButton: {
    background: '#007acc',
    border: 'none',
    borderRadius: 4,
    color: '#ffffff',
    padding: '6px 16px',
    cursor: 'pointer',
    fontSize: 12,
    fontWeight: 500,
  } as React.CSSProperties,
  statusDot: {
    width: 8,
    height: 8,
    borderRadius: '50%',
    flexShrink: 0,
  } as React.CSSProperties,
  statusText: {
    fontSize: 11,
    marginLeft: 4,
  } as React.CSSProperties,
  addButton: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 6,
    padding: '10px 16px',
    background: 'none',
    border: '1px dashed #3c3c3c',
    borderRadius: 6,
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    fontSize: 13,
    width: '100%',
    marginTop: 8,
  } as React.CSSProperties,
  description: {
    fontSize: 12,
    color: 'var(--text-muted)',
    lineHeight: 1.5,
    marginBottom: 16,
  } as React.CSSProperties,
  deleteButton: {
    background: 'none',
    border: 'none',
    color: '#f44747',
    cursor: 'pointer',
    fontSize: 11,
    marginLeft: 'auto',
    padding: '4px 8px',
    borderRadius: 3,
  } as React.CSSProperties,
  // Model library styles
  modelGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
    gap: 12,
    marginBottom: 24,
  } as React.CSSProperties,
  modelCard: {
    background: 'var(--bg-secondary)',
    border: '1px solid #3c3c3c',
    borderRadius: 6,
    padding: 14,
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 8,
  } as React.CSSProperties,
  modelCardHeader: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
  } as React.CSSProperties,
  modelCardName: {
    fontSize: 13,
    fontWeight: 600,
    color: 'var(--text-primary)',
    flex: 1,
  } as React.CSSProperties,
  modelCardBadge: {
    fontSize: 9,
    padding: '2px 6px',
    borderRadius: 8,
    fontWeight: 700,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.5px',
  } as React.CSSProperties,
  modelCardDesc: {
    fontSize: 11,
    color: 'var(--text-secondary)',
    lineHeight: 1.4,
  } as React.CSSProperties,
  modelCardMeta: {
    display: 'flex',
    gap: 8,
    flexWrap: 'wrap' as const,
  } as React.CSSProperties,
  modelCardMetaItem: {
    fontSize: 10,
    color: 'var(--text-muted)',
    background: 'var(--bg-primary)',
    padding: '2px 6px',
    borderRadius: 3,
    fontFamily: "'JetBrains Mono', monospace",
  } as React.CSSProperties,
  progressBar: {
    width: '100%',
    height: 4,
    background: 'var(--border-color)',
    borderRadius: 2,
    overflow: 'hidden' as const,
  } as React.CSSProperties,
  progressFill: {
    height: '100%',
    background: '#007acc',
    borderRadius: 2,
    transition: 'width 0.3s',
  } as React.CSSProperties,
  progressText: {
    fontSize: 10,
    color: 'var(--text-secondary)',
    fontFamily: "'JetBrains Mono', monospace",
  } as React.CSSProperties,
};

const PROVIDER_TYPE_STYLES: Record<string, React.CSSProperties> = {
  local: { background: '#4ec9b022', color: '#4ec9b0', border: '1px solid #4ec9b044' },
  openai: { background: '#569cd622', color: '#569cd6', border: '1px solid #569cd644' },
  anthropic: { background: '#c586c022', color: '#c586c0', border: '1px solid #c586c044' },
  'openai-compatible': { background: '#dcdcaa22', color: '#dcdcaa', border: '1px solid #dcdcaa44' },
};

const MODEL_OPTIONS: Record<string, string[]> = {
  local: ['Select GGUF model file...'],
  openai: ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo', 'o1', 'o1-mini'],
  anthropic: ['claude-sonnet-4-20250514', 'claude-3-5-haiku-20241022', 'claude-opus-4-20250514'],
  'openai-compatible': ['Enter model name...'],
};

function getDefaultProvider(type: LLMProviderConfig['type']): LLMProviderConfig {
  switch (type) {
    case 'local':
      return { id: `local-${Date.now()}`, name: 'Local Model (GGUF)', type: 'local', apiKey: '', baseUrl: '', model: '', enabled: false };
    case 'openai':
      return { id: `openai-${Date.now()}`, name: 'OpenAI', type: 'openai', apiKey: '', baseUrl: 'https://api.openai.com/v1', model: 'gpt-4o', enabled: false };
    case 'anthropic':
      return { id: `anthropic-${Date.now()}`, name: 'Anthropic', type: 'anthropic', apiKey: '', baseUrl: 'https://api.anthropic.com', model: 'claude-sonnet-4-20250514', enabled: false };
    case 'openai-compatible':
      return { id: `custom-${Date.now()}`, name: 'Custom Provider', type: 'openai-compatible', apiKey: '', baseUrl: 'http://localhost:11434/v1', model: '', enabled: false };
  }
}

function getStatusColor(status: ConnectionStatus): string {
  switch (status) {
    case 'connected': return '#4ec9b0';
    case 'failed': return '#f44747';
    case 'testing': return '#dcdcaa';
    default: return 'var(--text-muted)';
  }
}

function getStatusLabel(status: ConnectionStatus): string {
  switch (status) {
    case 'connected': return 'Connected';
    case 'failed': return 'Connection failed';
    case 'testing': return 'Testing...';
    default: return '';
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatSpeed(bps: number): string {
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(0)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
}

function formatEta(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

function ModelLibrary() {
  const [catalog, setCatalog] = useState<CatalogEntry[]>([]);
  const [downloadedModels, setDownloadedModels] = useState<DownloadedModel[]>([]);
  const [downloading, setDownloading] = useState<Set<string>>(new Set());
  const [startingModel, setStartingModel] = useState<string | null>(null);
  const [modelError, setModelError] = useState<string | null>(null);
  const { downloadProgress, setDownloadProgress, setLLMConfig, setLLMConnected } = useAppStore();

  const loadCatalog = useCallback(async () => {
    try {
      const result = await invoke<CatalogEntry[]>('list_model_catalog');
      setCatalog(result);
      // Cache catalog to localStorage for offline fallback
      localStorage.setItem('localcode-model-catalog', JSON.stringify(result));
    } catch (e) {
      console.error('Failed to load catalog:', e);
      // Fall back to cached catalog when offline
      try {
        const cached = localStorage.getItem('localcode-model-catalog');
        if (cached) {
          setCatalog(JSON.parse(cached));
        }
      } catch { /* ignore parse errors */ }
    }
  }, []);

  const loadDownloaded = useCallback(async () => {
    try {
      const result = await invoke<DownloadedModel[]>('list_downloaded_models');
      setDownloadedModels(result);
    } catch (e) {
      console.error('Failed to load downloaded models:', e);
    }
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadCatalog();
    loadDownloaded();

    const unlistenProgress = listen<{ catalog_id: string; downloaded_bytes: number; total_bytes: number; speed_bps: number; eta_seconds: number }>('model-download-progress', (event) => {
      setDownloadProgress(event.payload.catalog_id, {
        downloaded: event.payload.downloaded_bytes,
        total: event.payload.total_bytes,
        speed: event.payload.speed_bps,
        eta: event.payload.eta_seconds,
      });
    });

    const unlistenComplete = listen<{ catalog_id: string; path: string }>('model-download-complete', (event) => {
      setDownloadProgress(event.payload.catalog_id, null);
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(event.payload.catalog_id);
        return next;
      });
      loadCatalog();
      loadDownloaded();
    });

    return () => {
      unlistenProgress.then((f) => f());
      unlistenComplete.then((f) => f());
    };
  }, [loadCatalog, loadDownloaded, setDownloadProgress]);

  const handleDownload = useCallback(async (catalogId: string) => {
    setDownloading((prev) => new Set(prev).add(catalogId));
    try {
      await invoke('download_model', { catalogId });
    } catch (e) {
      console.error('Download failed:', e);
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(catalogId);
        return next;
      });
      setDownloadProgress(catalogId, null);
    }
  }, [setDownloadProgress]);

  const handleDelete = useCallback(async (catalogId: string) => {
    try {
      await invoke('delete_model', { catalogId });
      loadCatalog();
      loadDownloaded();
    } catch (e) {
      console.error('Delete failed:', e);
    }
  }, [loadCatalog, loadDownloaded]);

  const handleSelectModel = useCallback(async (model: DownloadedModel) => {
    setStartingModel(model.catalog_id);
    setModelError(null);
    try {
      await invoke('start_llm_server', { modelPath: model.path });
      setLLMConfig({ modelPath: model.path, modelName: model.name });
      setLLMConnected(true);
      setStartingModel(null);
    } catch (e: unknown) {
      console.error('Failed to start model:', e);
      setModelError(String(e));
      setStartingModel(null);
      setLLMConnected(false);
    }
  }, [setLLMConfig, setLLMConnected]);

  return (
    <div>
      <h3 style={styles.subsectionTitle}>Model Library</h3>
      <p style={styles.description}>
        Download and manage local AI models. Models are stored in ~/.localcode/models/.
      </p>

      {/* Status messages */}
      {startingModel && (
        <div style={{ padding: '10px 14px', background: '#007acc22', border: '1px solid #007acc44', borderRadius: 6, marginBottom: 12, fontSize: 12, color: '#569cd6', display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ width: 12, height: 12, border: '2px solid #569cd6', borderTopColor: 'transparent', borderRadius: '50%', animation: 'spin 1s linear infinite', display: 'inline-block' }} />
          Starting model server... This may take up to 30 seconds.
        </div>
      )}
      {modelError && (
        <div style={{ padding: '10px 14px', background: '#f4474722', border: '1px solid #f4474744', borderRadius: 6, marginBottom: 12, fontSize: 12, color: '#f44747' }}>
          Failed to start model: {modelError}
        </div>
      )}

      {/* Downloaded models dropdown */}
      {downloadedModels.length > 0 && (
        <div style={{ marginBottom: 16 }}>
          <div style={styles.fieldGroup}>
            <label style={styles.fieldLabel}>Active Model</label>
            <div style={{ display: 'flex', gap: 8 }}>
              <select
                style={{ ...styles.select, flex: 1 }}
                onChange={(e) => {
                  const model = downloadedModels.find((m) => m.catalog_id === e.target.value);
                  if (model) handleSelectModel(model);
                }}
                defaultValue=""
                disabled={!!startingModel}
              >
                <option value="" disabled>{startingModel ? 'Starting...' : 'Select a downloaded model...'}</option>
                {downloadedModels.map((m) => (
                  <option key={m.catalog_id} value={m.catalog_id}>
                    {m.name} ({formatBytes(m.size_bytes)})
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}

      {/* Catalog grid */}
      <div style={styles.modelGrid}>
        {catalog.map((entry) => {
          const progress = downloadProgress[entry.id];
          const isDownloading = downloading.has(entry.id);
          const isRecommended = entry.tags.includes('recommended');

          return (
            <div
              key={entry.id}
              style={{
                ...styles.modelCard,
                ...(entry.downloaded ? { borderColor: '#4ec9b044' } : {}),
              }}
            >
              <div style={styles.modelCardHeader}>
                <span style={styles.modelCardName}>{entry.name}</span>
                {isRecommended && (
                  <span style={{ ...styles.modelCardBadge, background: '#007acc33', color: '#007acc', border: '1px solid #007acc55' }}>
                    Recommended
                  </span>
                )}
                {entry.downloaded && (
                  <span style={{ ...styles.modelCardBadge, background: '#4ec9b022', color: '#4ec9b0', border: '1px solid #4ec9b044' }}>
                    Installed
                  </span>
                )}
              </div>

              <div style={styles.modelCardDesc}>{entry.description}</div>

              <div style={styles.modelCardMeta}>
                <span style={styles.modelCardMetaItem}>{entry.parameters}</span>
                <span style={styles.modelCardMetaItem}>{entry.quantization}</span>
                <span style={styles.modelCardMetaItem}>{formatBytes(entry.size_bytes)}</span>
                <span style={styles.modelCardMetaItem}>{(entry.context_length / 1024).toFixed(0)}K ctx</span>
              </div>

              {/* Progress bar */}
              {isDownloading && progress && (
                <div>
                  <div style={styles.progressBar}>
                    <div
                      style={{
                        ...styles.progressFill,
                        width: `${progress.total > 0 ? (progress.downloaded / progress.total) * 100 : 0}%`,
                      }}
                    />
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4 }}>
                    <span style={styles.progressText}>
                      {formatBytes(progress.downloaded)} / {formatBytes(progress.total)}
                    </span>
                    <span style={styles.progressText}>
                      {formatSpeed(progress.speed)} &middot; {formatEta(progress.eta)}
                    </span>
                  </div>
                </div>
              )}

              {/* Actions */}
              <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
                {!entry.downloaded && !isDownloading && (
                  <button
                    style={styles.saveButton}
                    onClick={() => handleDownload(entry.id)}
                    onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#1a8ad4'; }}
                    onMouseLeave={(e) => { (e.target as HTMLElement).style.background = '#007acc'; }}
                  >
                    Download
                  </button>
                )}
                {entry.downloaded && (
                  <>
                    <button
                      style={{
                        ...styles.saveButton,
                        ...(startingModel === entry.id ? { opacity: 0.7, cursor: 'wait' } : {}),
                      }}
                      disabled={!!startingModel}
                      onClick={() => {
                        const m = downloadedModels.find((dm) => dm.catalog_id === entry.id);
                        if (m) handleSelectModel(m);
                      }}
                      onMouseEnter={(e) => { if (!startingModel) (e.target as HTMLElement).style.background = '#1a8ad4'; }}
                      onMouseLeave={(e) => { (e.target as HTMLElement).style.background = '#007acc'; }}
                    >
                      {startingModel === entry.id ? 'Starting...' : 'Use Model'}
                    </button>
                    <button
                      style={styles.deleteButton}
                      onClick={() => handleDelete(entry.id)}
                      onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#f4474722'; }}
                      onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
                    >
                      Delete
                    </button>
                  </>
                )}
                {isDownloading && (
                  <span style={{ fontSize: 11, color: 'var(--text-secondary)', alignSelf: 'center' }}>
                    Downloading...
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ProviderCard({
  provider,
  onUpdate,
  onDelete,
}: {
  provider: LLMProviderConfig;
  onUpdate: (updates: Partial<LLMProviderConfig>) => void;
  onDelete: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('idle');
  const { setLLMConfig, setLLMConnected } = useAppStore();

  const handleTestConnection = useCallback(async () => {
    setConnectionStatus('testing');
    try {
      if (provider.type === 'local') {
        await invoke('start_llm_server', { modelPath: provider.model });
      } else {
        await invoke('test_llm_connection', {
          providerType: provider.type,
          apiKey: provider.apiKey,
          baseUrl: provider.baseUrl,
          model: provider.model,
        });
      }
      setConnectionStatus('connected');
      setLLMConnected(true);
      setLLMConfig({ modelName: provider.model || provider.name });
    } catch (err) {
      console.error('Connection test failed:', err);
      setConnectionStatus('failed');
      setLLMConnected(false);
    }
  }, [provider, setLLMConfig, setLLMConnected]);

  const handleSave = useCallback(async () => {
    try {
      await invoke('save_config', {
        key: `llm_provider_${provider.id}`,
        value: JSON.stringify(provider),
      });
    } catch (err) {
      console.error('Failed to save provider config:', err);
    }
  }, [provider]);

  const handleSelectModel = useCallback(async () => {
    if (provider.type !== 'local') return;
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        filters: [{ name: 'GGUF Models', extensions: ['gguf'] }],
      });
      if (selected) {
        onUpdate({ model: selected as string });
      }
    } catch (err) {
      console.error('Failed to select model:', err);
    }
  }, [provider.type, onUpdate]);

  return (
    <div
      style={{
        ...styles.providerCard,
        ...(provider.enabled ? styles.providerCardActive : {}),
      }}
    >
      <div
        style={{
          ...styles.providerHeader,
          ...(expanded ? styles.providerHeaderExpanded : {}),
        }}
        onClick={() => setExpanded(!expanded)}
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 16 16"
          fill="#969696"
          style={{
            transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)',
            transition: 'transform 0.15s',
            flexShrink: 0,
          }}
        >
          <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" fill="none" />
        </svg>

        <span style={styles.providerName}>{provider.name}</span>

        <span style={{ ...styles.providerType, ...PROVIDER_TYPE_STYLES[provider.type] }}>
          {provider.type}
        </span>

        {connectionStatus !== 'idle' && (
          <span
            style={{
              ...styles.statusDot,
              background: getStatusColor(connectionStatus),
            }}
            title={getStatusLabel(connectionStatus)}
          />
        )}

        <div
          style={{
            ...styles.enableToggle,
            background: provider.enabled ? '#007acc' : 'var(--border-color)',
          }}
          onClick={(e) => {
            e.stopPropagation();
            onUpdate({ enabled: !provider.enabled });
          }}
        >
          <div
            style={{
              ...styles.enableToggleKnob,
              position: 'absolute',
              top: 2,
              left: provider.enabled ? 18 : 2,
            }}
          />
        </div>
      </div>

      {expanded && (
        <div style={styles.providerBody}>
          <div style={styles.fieldGroup}>
            <label style={styles.fieldLabel}>Provider Name</label>
            <input
              style={styles.input}
              type="text"
              value={provider.name}
              onChange={(e) => onUpdate({ name: e.target.value })}
              placeholder="Provider name"
              onFocus={(e) => { (e.target as HTMLInputElement).style.borderColor = '#007acc'; }}
              onBlur={(e) => { (e.target as HTMLInputElement).style.borderColor = 'var(--border-color)'; }}
            />
          </div>

          {provider.type !== 'local' && (
            <div style={styles.fieldGroup}>
              <label style={styles.fieldLabel}>API Key</label>
              <input
                style={{ ...styles.input, ...styles.inputMono }}
                type="password"
                value={provider.apiKey}
                onChange={(e) => onUpdate({ apiKey: e.target.value })}
                placeholder="sk-..."
                onFocus={(e) => { (e.target as HTMLInputElement).style.borderColor = '#007acc'; }}
                onBlur={(e) => { (e.target as HTMLInputElement).style.borderColor = 'var(--border-color)'; }}
              />
            </div>
          )}

          {(provider.type === 'openai-compatible' || provider.type === 'openai' || provider.type === 'anthropic') && (
            <div style={styles.fieldGroup}>
              <label style={styles.fieldLabel}>Base URL</label>
              <input
                style={{ ...styles.input, ...styles.inputMono }}
                type="text"
                value={provider.baseUrl}
                onChange={(e) => onUpdate({ baseUrl: e.target.value })}
                placeholder="https://api.example.com/v1"
                onFocus={(e) => { (e.target as HTMLInputElement).style.borderColor = '#007acc'; }}
                onBlur={(e) => { (e.target as HTMLInputElement).style.borderColor = 'var(--border-color)'; }}
              />
            </div>
          )}

          <div style={styles.fieldGroup}>
            <label style={styles.fieldLabel}>Model</label>
            {provider.type === 'local' ? (
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                <input
                  style={{ ...styles.input, ...styles.inputMono, flex: 1 }}
                  type="text"
                  value={provider.model}
                  onChange={(e) => onUpdate({ model: e.target.value })}
                  placeholder="Path to .gguf model file"
                  readOnly
                  onFocus={(e) => { (e.target as HTMLInputElement).style.borderColor = '#007acc'; }}
                  onBlur={(e) => { (e.target as HTMLInputElement).style.borderColor = 'var(--border-color)'; }}
                />
                <button
                  style={styles.testButton}
                  onClick={handleSelectModel}
                  onMouseEnter={(e) => { (e.target as HTMLElement).style.borderColor = 'var(--text-secondary)'; }}
                  onMouseLeave={(e) => { (e.target as HTMLElement).style.borderColor = 'var(--border-color)'; }}
                >
                  Browse
                </button>
              </div>
            ) : provider.type === 'openai-compatible' ? (
              <input
                style={{ ...styles.input, ...styles.inputMono }}
                type="text"
                value={provider.model}
                onChange={(e) => onUpdate({ model: e.target.value })}
                placeholder="model-name"
                onFocus={(e) => { (e.target as HTMLInputElement).style.borderColor = '#007acc'; }}
                onBlur={(e) => { (e.target as HTMLInputElement).style.borderColor = 'var(--border-color)'; }}
              />
            ) : (
              <select
                style={styles.select}
                value={provider.model}
                onChange={(e) => onUpdate({ model: e.target.value })}
              >
                {MODEL_OPTIONS[provider.type]?.map((model) => (
                  <option key={model} value={model}>{model}</option>
                ))}
              </select>
            )}
          </div>

          <div style={styles.actions}>
            <button
              style={styles.testButton}
              onClick={(e) => { e.stopPropagation(); handleTestConnection(); }}
              disabled={connectionStatus === 'testing'}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.borderColor = 'var(--text-secondary)'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.borderColor = 'var(--border-color)'; }}
            >
              {connectionStatus === 'testing' ? (
                <>
                  <span style={{ ...styles.statusDot, background: '#dcdcaa' }} />
                  Testing...
                </>
              ) : (
                <>
                  <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M5.56 14.18l4.67-4.24.71.71-5.38 4.89-.02-.01-.7.7L0 11.39l.7-.7 4.86 3.49z" />
                  </svg>
                  Test Connection
                </>
              )}
            </button>
            <button
              style={styles.saveButton}
              onClick={(e) => { e.stopPropagation(); handleSave(); }}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#1a8ad4'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = '#007acc'; }}
            >
              Save
            </button>

            {connectionStatus !== 'idle' && (
              <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <span style={{ ...styles.statusDot, background: getStatusColor(connectionStatus) }} />
                <span style={{ ...styles.statusText, color: getStatusColor(connectionStatus) }}>
                  {getStatusLabel(connectionStatus)}
                </span>
              </span>
            )}

            <button
              style={styles.deleteButton}
              onClick={(e) => { e.stopPropagation(); onDelete(); }}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#f4474722'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
            >
              Remove
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default function LLMSettings() {
  const [providers, setProviders] = useState<LLMProviderConfig[]>([
    {
      id: 'local-default',
      name: 'Local Model (GGUF)',
      type: 'local',
      apiKey: '',
      baseUrl: '',
      model: '',
      enabled: false,
    },
  ]);
  const [showAddMenu, setShowAddMenu] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const config = await invoke<string>('load_config', { key: 'llm_providers' });
        if (config) {
          const parsed = JSON.parse(config);
          if (Array.isArray(parsed) && parsed.length > 0) {
            setProviders(parsed);
          }
        }
      } catch {
        // No saved config, use defaults
      }
    })();
  }, []);

  const handleUpdateProvider = useCallback((id: string, updates: Partial<LLMProviderConfig>) => {
    setProviders((prev) =>
      prev.map((p) => (p.id === id ? { ...p, ...updates } : p))
    );
  }, []);

  const handleDeleteProvider = useCallback((id: string) => {
    setProviders((prev) => prev.filter((p) => p.id !== id));
  }, []);

  const handleAddProvider = useCallback((type: LLMProviderConfig['type']) => {
    const newProvider = getDefaultProvider(type);
    setProviders((prev) => [...prev, newProvider]);
    setShowAddMenu(false);
  }, []);

  return (
    <div style={styles.container}>
      <h2 style={styles.sectionTitle}>LLM Providers</h2>

      <div style={styles.description}>
        Configure AI model providers for code completion, chat, and agent features.
        LocalCode supports local GGUF models, OpenAI, Anthropic, and any OpenAI-compatible API.
      </div>

      {/* Model Library - Automated Download */}
      <ModelLibrary />

      {/* Cloud / API Providers */}
      <h3 style={styles.subsectionTitle}>Cloud & API Providers</h3>

      {providers.map((provider) => (
        <ProviderCard
          key={provider.id}
          provider={provider}
          onUpdate={(updates) => handleUpdateProvider(provider.id, updates)}
          onDelete={() => handleDeleteProvider(provider.id)}
        />
      ))}

      <div style={{ position: 'relative' }}>
        <button
          style={styles.addButton}
          onClick={() => setShowAddMenu(!showAddMenu)}
          onMouseEnter={(e) => { (e.target as HTMLElement).style.borderColor = 'var(--text-secondary)'; }}
          onMouseLeave={(e) => { (e.target as HTMLElement).style.borderColor = 'var(--border-color)'; }}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M14 7v1H8v6H7V8H1V7h6V1h1v6h6z" />
          </svg>
          Add Provider
        </button>

        {showAddMenu && (
          <div
            style={{
              position: 'absolute',
              top: '100%',
              left: '50%',
              transform: 'translateX(-50%)',
              marginTop: 4,
              background: 'var(--bg-secondary)',
              border: '1px solid #3c3c3c',
              borderRadius: 4,
              boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
              zIndex: 10,
              minWidth: 200,
              overflow: 'hidden',
            }}
          >
            {[
              { type: 'local' as const, label: 'Local Model (GGUF)', color: '#4ec9b0' },
              { type: 'openai' as const, label: 'OpenAI', color: '#569cd6' },
              { type: 'anthropic' as const, label: 'Anthropic', color: '#c586c0' },
              { type: 'openai-compatible' as const, label: 'OpenAI-Compatible', color: '#dcdcaa' },
            ].map((item) => (
              <div
                key={item.type}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '8px 12px',
                  cursor: 'pointer',
                  fontSize: 13,
                  color: 'var(--text-primary)',
                }}
                onClick={() => handleAddProvider(item.type)}
                onMouseEnter={(e) => {
                  (e.currentTarget as HTMLElement).style.background = 'var(--bg-hover)';
                }}
                onMouseLeave={(e) => {
                  (e.currentTarget as HTMLElement).style.background = 'transparent';
                }}
              >
                <span style={{ width: 8, height: 8, borderRadius: '50%', background: item.color, flexShrink: 0 }} />
                {item.label}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
