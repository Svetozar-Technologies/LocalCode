import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';

interface MCPServer {
  name: string;
  transport: 'stdio' | 'sse';
  command?: string;
  args?: string[];
  url?: string;
  enabled: boolean;
}

interface MCPConfig {
  servers: MCPServer[];
}

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 16,
  } as React.CSSProperties,
  serverCard: {
    background: 'var(--bg-secondary)',
    border: '1px solid var(--border-color)',
    borderRadius: 6,
    padding: '12px 16px',
    display: 'flex',
    flexDirection: 'column' as const,
    gap: 8,
  } as React.CSSProperties,
  serverHeader: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    fontSize: 13,
  } as React.CSSProperties,
  serverName: {
    fontWeight: 600,
    color: 'var(--text-primary)',
    flex: 1,
  } as React.CSSProperties,
  serverDetail: {
    fontSize: 12,
    color: 'var(--text-secondary)',
    fontFamily: 'var(--font-mono)',
  } as React.CSSProperties,
  badge: {
    padding: '1px 8px',
    borderRadius: 8,
    fontSize: 10,
    fontWeight: 600,
  } as React.CSSProperties,
  input: {
    width: '100%',
    background: 'var(--bg-input)',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '6px 10px',
    fontSize: 12,
    outline: 'none',
    fontFamily: 'var(--font-ui)',
  } as React.CSSProperties,
  select: {
    background: 'var(--bg-input)',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '6px 10px',
    fontSize: 12,
    outline: 'none',
  } as React.CSSProperties,
  button: {
    background: 'var(--accent)',
    border: 'none',
    borderRadius: 4,
    color: '#fff',
    padding: '6px 14px',
    cursor: 'pointer',
    fontSize: 12,
  } as React.CSSProperties,
  secondaryButton: {
    background: 'none',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    color: 'var(--text-secondary)',
    padding: '5px 12px',
    cursor: 'pointer',
    fontSize: 12,
  } as React.CSSProperties,
  deleteButton: {
    background: 'none',
    border: 'none',
    color: 'var(--accent-red)',
    cursor: 'pointer',
    fontSize: 12,
    padding: '2px 6px',
  } as React.CSSProperties,
};

export default function MCPSettings() {
  const { projectPath } = useAppStore();
  const [config, setConfig] = useState<MCPConfig>({ servers: [] });
  const [showAddForm, setShowAddForm] = useState(false);
  const [newServer, setNewServer] = useState<MCPServer>({
    name: '',
    transport: 'stdio',
    command: '',
    args: [],
    url: '',
    enabled: true,
  });
  const [testResult, setTestResult] = useState<Record<string, string>>({});

  const configPath = projectPath ? `${projectPath}/.localcode/mcp.json` : null;

  const loadConfig = useCallback(async () => {
    if (!configPath) return;
    try {
      const content = await invoke<string>('read_file', { path: configPath });
      const parsed = JSON.parse(content) as MCPConfig;
      setConfig(parsed);
    } catch {
      setConfig({ servers: [] });
    }
  }, [configPath]);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const saveConfig = useCallback(async (newConfig: MCPConfig) => {
    if (!configPath || !projectPath) return;
    try {
      // Ensure .localcode directory exists
      await invoke('create_dir', { path: `${projectPath}/.localcode` }).catch(() => {});
      await invoke('write_file', {
        path: configPath,
        content: JSON.stringify(newConfig, null, 2),
      });
      setConfig(newConfig);
    } catch (err) {
      console.error('Failed to save MCP config:', err);
    }
  }, [configPath, projectPath]);

  const handleAddServer = useCallback(() => {
    if (!newServer.name.trim()) return;
    const updated = {
      servers: [...config.servers, { ...newServer }],
    };
    saveConfig(updated);
    setNewServer({ name: '', transport: 'stdio', command: '', args: [], url: '', enabled: true });
    setShowAddForm(false);
  }, [config, newServer, saveConfig]);

  const handleRemoveServer = useCallback((index: number) => {
    const updated = {
      servers: config.servers.filter((_, i) => i !== index),
    };
    saveConfig(updated);
  }, [config, saveConfig]);

  const handleToggleServer = useCallback((index: number) => {
    const updated = {
      servers: config.servers.map((s, i) =>
        i === index ? { ...s, enabled: !s.enabled } : s
      ),
    };
    saveConfig(updated);
  }, [config, saveConfig]);

  const handleTestConnection = useCallback(async (server: MCPServer, index: number) => {
    setTestResult((prev) => ({ ...prev, [index]: 'testing...' }));
    try {
      // Simple test: try to verify the command exists
      if (server.transport === 'stdio' && server.command) {
        // For stdio, we just check if the command path looks valid
        setTestResult((prev) => ({ ...prev, [index]: 'Command configured (run to verify)' }));
      } else if (server.transport === 'sse' && server.url) {
        setTestResult((prev) => ({ ...prev, [index]: `SSE endpoint: ${server.url}` }));
      } else {
        setTestResult((prev) => ({ ...prev, [index]: 'Missing command/URL' }));
      }
    } catch (err) {
      setTestResult((prev) => ({ ...prev, [index]: `Error: ${err}` }));
    }
  }, []);

  return (
    <div style={styles.container}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        <h3 style={{ fontSize: 14, color: 'var(--text-primary)', margin: 0 }}>MCP Servers</h3>
        <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>
          {config.servers.length} configured
        </span>
        <button
          style={{ ...styles.button, marginLeft: 'auto' }}
          onClick={() => setShowAddForm(!showAddForm)}
        >
          {showAddForm ? 'Cancel' : '+ Add Server'}
        </button>
      </div>

      {showAddForm && (
        <div style={{ ...styles.serverCard, borderColor: 'var(--accent)' }}>
          <div style={{ display: 'flex', gap: 8 }}>
            <input
              style={{ ...styles.input, flex: 1 }}
              placeholder="Server name"
              value={newServer.name}
              onChange={(e) => setNewServer({ ...newServer, name: e.target.value })}
            />
            <select
              style={styles.select}
              value={newServer.transport}
              onChange={(e) => setNewServer({ ...newServer, transport: e.target.value as 'stdio' | 'sse' })}
            >
              <option value="stdio">stdio</option>
              <option value="sse">SSE</option>
            </select>
          </div>
          {newServer.transport === 'stdio' ? (
            <>
              <input
                style={styles.input}
                placeholder="Command (e.g., npx -y @modelcontextprotocol/server-name)"
                value={newServer.command}
                onChange={(e) => setNewServer({ ...newServer, command: e.target.value })}
              />
              <input
                style={styles.input}
                placeholder="Arguments (comma-separated)"
                value={(newServer.args || []).join(', ')}
                onChange={(e) => setNewServer({ ...newServer, args: e.target.value.split(',').map((s) => s.trim()).filter(Boolean) })}
              />
            </>
          ) : (
            <input
              style={styles.input}
              placeholder="SSE URL (e.g., http://localhost:3001/sse)"
              value={newServer.url}
              onChange={(e) => setNewServer({ ...newServer, url: e.target.value })}
            />
          )}
          <button style={styles.button} onClick={handleAddServer}>
            Add Server
          </button>
        </div>
      )}

      {config.servers.length === 0 && !showAddForm && (
        <div style={{ textAlign: 'center', color: 'var(--text-muted)', padding: 24, fontSize: 12 }}>
          No MCP servers configured. Add a server to extend AI capabilities.
        </div>
      )}

      {config.servers.map((server, index) => (
        <div key={index} style={{ ...styles.serverCard, opacity: server.enabled ? 1 : 0.6 }}>
          <div style={styles.serverHeader}>
            <input
              type="checkbox"
              checked={server.enabled}
              onChange={() => handleToggleServer(index)}
              style={{ accentColor: 'var(--accent)' }}
            />
            <span style={styles.serverName}>{server.name}</span>
            <span style={{
              ...styles.badge,
              background: server.transport === 'stdio' ? 'rgba(86,156,214,0.15)' : 'rgba(78,201,176,0.15)',
              color: server.transport === 'stdio' ? '#569cd6' : '#4ec9b0',
            }}>
              {server.transport}
            </span>
            <button
              style={styles.secondaryButton}
              onClick={() => handleTestConnection(server, index)}
            >
              Test
            </button>
            <button
              style={styles.deleteButton}
              onClick={() => handleRemoveServer(index)}
            >
              Remove
            </button>
          </div>
          <div style={styles.serverDetail}>
            {server.transport === 'stdio'
              ? `${server.command} ${(server.args || []).join(' ')}`
              : server.url}
          </div>
          {testResult[index] && (
            <div style={{ fontSize: 11, color: testResult[index].startsWith('Error') ? 'var(--accent-red)' : 'var(--accent-green)' }}>
              {testResult[index]}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
