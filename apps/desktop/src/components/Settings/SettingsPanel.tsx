import { useState } from 'react';
import { useAppStore } from '../../stores/appStore';
import LLMSettings from './LLMSettings';
import MCPSettings from './MCPSettings';

type SettingsSection = 'general' | 'editor' | 'llm' | 'agent' | 'keybindings' | 'mcp' | 'remote' | 'collaboration';

interface SettingsSectionItem {
  id: SettingsSection;
  label: string;
  icon: React.ReactNode;
}

const styles = {
  container: {
    display: 'flex',
    height: '100%',
    background: 'var(--bg-primary)',
    fontFamily: 'var(--font-ui)',
  } as React.CSSProperties,
  sidebar: {
    width: 200,
    minWidth: 200,
    background: 'var(--bg-secondary)',
    borderRight: '1px solid var(--border-color)',
    display: 'flex',
    flexDirection: 'column' as const,
    overflow: 'auto',
  } as React.CSSProperties,
  sidebarHeader: {
    padding: '16px 16px 12px',
    fontSize: 13,
    fontWeight: 600,
    color: 'var(--text-primary)',
    borderBottom: '1px solid var(--border-color)',
  } as React.CSSProperties,
  navItem: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    padding: '8px 16px',
    cursor: 'pointer',
    fontSize: 13,
    color: 'var(--text-secondary)',
    borderLeft: '2px solid transparent',
    transition: 'color 0.1s, background 0.1s',
  } as React.CSSProperties,
  navItemActive: {
    color: 'var(--text-primary)',
    background: 'var(--bg-hover)',
    borderLeftColor: 'var(--accent)',
  } as React.CSSProperties,
  content: {
    flex: 1,
    overflow: 'auto',
    padding: '24px 32px',
  } as React.CSSProperties,
  sectionTitle: {
    fontSize: 18,
    fontWeight: 600,
    color: 'var(--text-primary)',
    marginBottom: 20,
    paddingBottom: 8,
    borderBottom: '1px solid var(--border-color)',
  } as React.CSSProperties,
  settingGroup: {
    marginBottom: 24,
  } as React.CSSProperties,
  settingLabel: {
    fontSize: 13,
    fontWeight: 500,
    color: 'var(--text-primary)',
    marginBottom: 4,
  } as React.CSSProperties,
  settingDescription: {
    fontSize: 12,
    color: 'var(--text-muted)',
    marginBottom: 8,
    lineHeight: 1.5,
  } as React.CSSProperties,
  input: {
    width: '100%',
    maxWidth: 400,
    background: 'var(--bg-input)',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '6px 10px',
    fontSize: 13,
    outline: 'none',
    fontFamily: 'var(--font-ui)',
  } as React.CSSProperties,
  select: {
    background: 'var(--bg-input)',
    border: '1px solid var(--border-color)',
    borderRadius: 4,
    color: 'var(--text-primary)',
    padding: '6px 10px',
    fontSize: 13,
    outline: 'none',
    minWidth: 200,
  } as React.CSSProperties,
  checkbox: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    cursor: 'pointer',
    fontSize: 13,
    color: 'var(--text-primary)',
  } as React.CSSProperties,
  checkboxInput: {
    accentColor: 'var(--accent)',
    width: 14,
    height: 14,
  } as React.CSSProperties,
  slider: {
    width: '100%',
    maxWidth: 300,
    accentColor: 'var(--accent)',
  } as React.CSSProperties,
  sliderValue: {
    fontSize: 12,
    color: 'var(--text-secondary)',
    marginLeft: 8,
    fontFamily: 'var(--font-mono)',
  } as React.CSSProperties,
  keybindingRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '8px 0',
    borderBottom: '1px solid var(--bg-tertiary)',
    fontSize: 13,
  } as React.CSSProperties,
  keybindingAction: {
    color: 'var(--text-primary)',
  } as React.CSSProperties,
  kbd: {
    background: 'var(--bg-tertiary)',
    border: '1px solid var(--border-color)',
    borderRadius: 3,
    padding: '2px 8px',
    fontFamily: 'var(--font-mono)',
    fontSize: 12,
    color: 'var(--text-primary)',
  } as React.CSSProperties,
};

const SECTIONS: SettingsSectionItem[] = [
  {
    id: 'general',
    label: 'General',
    icon: (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 00.12-.61l-1.92-3.32a.49.49 0 00-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 00-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.07.62-.07.94s.02.64.07.94l-2.03 1.58a.49.49 0 00-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z" />
      </svg>
    ),
  },
  {
    id: 'editor',
    label: 'Editor',
    icon: (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M13.71 4.29l-3-3L10 1H4L3 2v12l1 1h9l1-1V5l-.29-.71zM13 14H4V2h5v4h4v8z" />
      </svg>
    ),
  },
  {
    id: 'llm',
    label: 'LLM Providers',
    icon: (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93z" />
      </svg>
    ),
  },
  {
    id: 'agent',
    label: 'Agent',
    icon: (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M1 2.795l.783-.419 5.371 3.581v.838l-5.371 3.581L1 9.957v-7.162zm0 8.205h14v1H1v-1z" />
      </svg>
    ),
  },
  {
    id: 'keybindings',
    label: 'Keybindings',
    icon: (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M14 3H2L1 4v8l1 1h12l1-1V4l-1-1zm0 9H2V4h12v8zM3 5h2v2H3V5zm0 3h2v2H3V8zm3-3h2v2H6V5zm0 3h2v2H6V8zm3-3h2v2H9V5zm0 3h2v2H9V8zm3-3h1v2h-1V5zm0 3h1v2h-1V8z" />
      </svg>
    ),
  },
  {
    id: 'mcp',
    label: 'MCP Servers',
    icon: (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M14 3.5l-5-2-5 2v9l5 2 5-2v-9zM9 12.63l-4-1.6V4.37l4 1.6v6.66zm1-6.66l4-1.6v6.66l-4 1.6V5.97z" />
      </svg>
    ),
  },
  {
    id: 'remote',
    label: 'Remote Dev',
    icon: (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M14.5 2h-13l-.5.5v11l.5.5h13l.5-.5V2.5l-.5-.5zM14 13H2V3h12v10zM4 11h3v1H4v-1zm5 0h3v1H9v-1z" />
      </svg>
    ),
  },
  {
    id: 'collaboration',
    label: 'Collaboration',
    icon: (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M12 6a3 3 0 100-6 3 3 0 000 6zm-8 0a3 3 0 100-6 3 3 0 000 6zm8 2c-2.33 0-7 1.17-7 3.5V14h14v-2.5C20 9.17 14.33 8 12 8zM4 8C1.67 8-3 9.17-3 11.5V14h7v-2.5c0-.72.22-1.34.59-1.87C3.48 8.79 1.93 8 0 8z" />
      </svg>
    ),
  },
];

const KEYBINDINGS = [
  { action: 'Open File', shortcut: 'Cmd+P' },
  { action: 'Save File', shortcut: 'Cmd+S' },
  { action: 'Find in File', shortcut: 'Cmd+F' },
  { action: 'Find in Files', shortcut: 'Cmd+Shift+F' },
  { action: 'Toggle Terminal', shortcut: 'Cmd+`' },
  { action: 'Toggle Sidebar', shortcut: 'Cmd+B' },
  { action: 'AI Chat', shortcut: 'Cmd+I' },
  { action: 'Inline Edit', shortcut: 'Cmd+K' },
  { action: 'Split Editor', shortcut: 'Cmd+\\' },
  { action: 'Composer', shortcut: 'Cmd+Shift+I' },
  { action: 'Go to Line', shortcut: 'Ctrl+G' },
  { action: 'Command Palette', shortcut: 'Cmd+Shift+P' },
  { action: 'Close Tab', shortcut: 'Cmd+W' },
  { action: 'New Terminal', shortcut: 'Ctrl+Shift+`' },
  { action: 'Toggle Word Wrap', shortcut: 'Alt+Z' },
];

function GeneralSettings() {
  const { theme, setTheme, autoSave, setAutoSave, autoSaveDelay, setAutoSaveDelay } = useAppStore();

  return (
    <>
      <h2 style={styles.sectionTitle}>General</h2>

      <div style={styles.settingGroup}>
        <div style={styles.settingLabel}>Theme</div>
        <div style={styles.settingDescription}>Select the color theme for the editor and UI.</div>
        <select
          style={styles.select}
          value={theme}
          onChange={(e) => setTheme(e.target.value)}
        >
          <option value="dark">Dark+ (Default)</option>
          <option value="light">Light+</option>
          <option value="monokai">Monokai</option>
          <option value="solarized">Solarized Dark</option>
        </select>
      </div>

      <div style={styles.settingGroup}>
        <label style={styles.checkbox}>
          <input
            type="checkbox"
            style={styles.checkboxInput}
            checked={autoSave}
            onChange={(e) => setAutoSave(e.target.checked)}
          />
          Auto Save
        </label>
        <div style={styles.settingDescription}>
          Automatically save files after a delay when you stop typing.
        </div>
        {autoSave && (
          <div style={{ display: 'flex', alignItems: 'center', marginTop: 6 }}>
            <input
              type="range"
              style={styles.slider}
              min={500}
              max={5000}
              step={100}
              value={autoSaveDelay}
              onChange={(e) => setAutoSaveDelay(Number(e.target.value))}
            />
            <span style={styles.sliderValue}>{autoSaveDelay}ms</span>
          </div>
        )}
      </div>
    </>
  );
}

function EditorSettings() {
  const [fontSize, setFontSize] = useState(14);
  const [tabSize, setTabSize] = useState(2);
  const [wordWrap, setWordWrap] = useState(false);
  const [minimap, setMinimap] = useState(true);
  const [bracketPairs, setBracketPairs] = useState(true);
  const [fontLigatures, setFontLigatures] = useState(true);
  const [lineNumbers, setLineNumbers] = useState('on');
  const [renderWhitespace, setRenderWhitespace] = useState('selection');

  return (
    <>
      <h2 style={styles.sectionTitle}>Editor</h2>

      <div style={styles.settingGroup}>
        <div style={styles.settingLabel}>Font Size</div>
        <div style={styles.settingDescription}>Controls the font size in pixels.</div>
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <input
            type="range"
            style={styles.slider}
            min={10}
            max={24}
            value={fontSize}
            onChange={(e) => setFontSize(Number(e.target.value))}
          />
          <span style={styles.sliderValue}>{fontSize}px</span>
        </div>
      </div>

      <div style={styles.settingGroup}>
        <div style={styles.settingLabel}>Tab Size</div>
        <div style={styles.settingDescription}>The number of spaces a tab is equal to.</div>
        <select
          style={styles.select}
          value={tabSize}
          onChange={(e) => setTabSize(Number(e.target.value))}
        >
          <option value={2}>2</option>
          <option value={4}>4</option>
          <option value={8}>8</option>
        </select>
      </div>

      <div style={styles.settingGroup}>
        <div style={styles.settingLabel}>Line Numbers</div>
        <div style={styles.settingDescription}>Controls the display of line numbers.</div>
        <select
          style={styles.select}
          value={lineNumbers}
          onChange={(e) => setLineNumbers(e.target.value)}
        >
          <option value="on">On</option>
          <option value="off">Off</option>
          <option value="relative">Relative</option>
        </select>
      </div>

      <div style={styles.settingGroup}>
        <div style={styles.settingLabel}>Render Whitespace</div>
        <div style={styles.settingDescription}>Controls how whitespace is rendered.</div>
        <select
          style={styles.select}
          value={renderWhitespace}
          onChange={(e) => setRenderWhitespace(e.target.value)}
        >
          <option value="none">None</option>
          <option value="boundary">Boundary</option>
          <option value="selection">Selection</option>
          <option value="all">All</option>
        </select>
      </div>

      <div style={styles.settingGroup}>
        <label style={styles.checkbox}>
          <input type="checkbox" style={styles.checkboxInput} checked={wordWrap} onChange={(e) => setWordWrap(e.target.checked)} />
          Word Wrap
        </label>
        <div style={styles.settingDescription}>Controls if lines should wrap or scroll horizontally.</div>
      </div>

      <div style={styles.settingGroup}>
        <label style={styles.checkbox}>
          <input type="checkbox" style={styles.checkboxInput} checked={minimap} onChange={(e) => setMinimap(e.target.checked)} />
          Minimap
        </label>
        <div style={styles.settingDescription}>Controls whether the minimap is shown.</div>
      </div>

      <div style={styles.settingGroup}>
        <label style={styles.checkbox}>
          <input type="checkbox" style={styles.checkboxInput} checked={bracketPairs} onChange={(e) => setBracketPairs(e.target.checked)} />
          Bracket Pair Colorization
        </label>
        <div style={styles.settingDescription}>Controls whether bracket pair colorization is enabled.</div>
      </div>

      <div style={styles.settingGroup}>
        <label style={styles.checkbox}>
          <input type="checkbox" style={styles.checkboxInput} checked={fontLigatures} onChange={(e) => setFontLigatures(e.target.checked)} />
          Font Ligatures
        </label>
        <div style={styles.settingDescription}>Enables font ligatures for supported fonts.</div>
      </div>
    </>
  );
}

function AgentSettings() {
  const [maxSteps, setMaxSteps] = useState(10);
  const [autoApprove, setAutoApprove] = useState(false);
  const [sandboxEnabled, setSandboxEnabled] = useState(true);
  const [allowedTools, setAllowedTools] = useState<string[]>([
    'read_file', 'write_file', 'search_content', 'run_command',
  ]);

  const ALL_TOOLS = [
    { id: 'read_file', label: 'Read File' },
    { id: 'write_file', label: 'Write File' },
    { id: 'search_content', label: 'Search Content' },
    { id: 'run_command', label: 'Run Command' },
    { id: 'git_commit', label: 'Git Commit' },
    { id: 'git_add', label: 'Git Add' },
    { id: 'create_file', label: 'Create File' },
    { id: 'delete_file', label: 'Delete File' },
  ];

  const toggleTool = (toolId: string) => {
    setAllowedTools((prev) =>
      prev.includes(toolId) ? prev.filter((t) => t !== toolId) : [...prev, toolId]
    );
  };

  return (
    <>
      <h2 style={styles.sectionTitle}>Agent</h2>

      <div style={styles.settingGroup}>
        <div style={styles.settingLabel}>Max Steps</div>
        <div style={styles.settingDescription}>Maximum number of steps the agent can take per task.</div>
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <input type="range" style={styles.slider} min={1} max={50} value={maxSteps} onChange={(e) => setMaxSteps(Number(e.target.value))} />
          <span style={styles.sliderValue}>{maxSteps}</span>
        </div>
      </div>

      <div style={styles.settingGroup}>
        <label style={styles.checkbox}>
          <input type="checkbox" style={styles.checkboxInput} checked={autoApprove} onChange={(e) => setAutoApprove(e.target.checked)} />
          Auto-Approve Tool Calls
        </label>
        <div style={styles.settingDescription}>Automatically approve agent tool calls without confirmation. Use with caution.</div>
      </div>

      <div style={styles.settingGroup}>
        <label style={styles.checkbox}>
          <input type="checkbox" style={styles.checkboxInput} checked={sandboxEnabled} onChange={(e) => setSandboxEnabled(e.target.checked)} />
          Sandbox Mode
        </label>
        <div style={styles.settingDescription}>Run agent commands in a sandboxed environment for safety.</div>
      </div>

      <div style={styles.settingGroup}>
        <div style={styles.settingLabel}>Allowed Tools</div>
        <div style={styles.settingDescription}>Select which tools the agent is allowed to use.</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginTop: 8 }}>
          {ALL_TOOLS.map((tool) => (
            <label key={tool.id} style={styles.checkbox}>
              <input type="checkbox" style={styles.checkboxInput} checked={allowedTools.includes(tool.id)} onChange={() => toggleTool(tool.id)} />
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12 }}>{tool.label}</span>
            </label>
          ))}
        </div>
      </div>
    </>
  );
}

function KeybindingsSettings() {
  return (
    <>
      <h2 style={styles.sectionTitle}>Keybindings</h2>
      <div style={styles.settingDescription}>
        Default keyboard shortcuts. Custom keybinding support coming soon.
      </div>
      <div style={{ marginTop: 16 }}>
        {KEYBINDINGS.map((kb) => (
          <div key={kb.action} style={styles.keybindingRow}>
            <span style={styles.keybindingAction}>{kb.action}</span>
            <kbd style={styles.kbd}>{kb.shortcut}</kbd>
          </div>
        ))}
      </div>
    </>
  );
}

function MCPSettingsSection() {
  return (
    <>
      <h2 style={styles.sectionTitle}>MCP Servers</h2>
      <div style={styles.settingDescription}>
        Configure Model Context Protocol (MCP) servers to extend AI capabilities with external tools and data sources.
      </div>
      <div style={{ marginTop: 16 }}>
        <MCPSettings />
      </div>
    </>
  );
}

function RemoteDevSettings() {
  return (
    <>
      <h2 style={styles.sectionTitle}>Remote Development</h2>
      <div style={{
        padding: '32px 24px',
        textAlign: 'center',
        color: 'var(--text-muted)',
        fontSize: 13,
      }}>
        <svg width="48" height="48" viewBox="0 0 16 16" fill="var(--text-muted)" style={{ opacity: 0.3, marginBottom: 12 }}>
          <path d="M14.5 2h-13l-.5.5v11l.5.5h13l.5-.5V2.5l-.5-.5zM14 13H2V3h12v10z" />
        </svg>
        <p style={{ fontWeight: 600, marginBottom: 8 }}>Coming Soon</p>
        <p>Connect to remote machines via SSH and develop directly on them. Remote development support is planned for a future release.</p>
        <div style={{ marginTop: 20, opacity: 0.5 }}>
          <div style={styles.settingGroup}>
            <div style={styles.settingLabel}>SSH Host</div>
            <input style={{ ...styles.input, opacity: 0.5 }} placeholder="user@hostname" disabled />
          </div>
          <div style={styles.settingGroup}>
            <div style={styles.settingLabel}>Port</div>
            <input style={{ ...styles.input, opacity: 0.5, maxWidth: 100 }} placeholder="22" disabled />
          </div>
        </div>
      </div>
    </>
  );
}

function CollaborationSettings() {
  return (
    <>
      <h2 style={styles.sectionTitle}>Collaboration</h2>
      <div style={{
        padding: '32px 24px',
        textAlign: 'center',
        color: 'var(--text-muted)',
        fontSize: 13,
      }}>
        <svg width="48" height="48" viewBox="0 0 16 16" fill="var(--text-muted)" style={{ opacity: 0.3, marginBottom: 12 }}>
          <path d="M12 6a3 3 0 100-6 3 3 0 000 6zm-8 0a3 3 0 100-6 3 3 0 000 6z" />
        </svg>
        <p style={{ fontWeight: 600, marginBottom: 8 }}>Coming Soon</p>
        <p>Real-time collaboration with team members. Share editing sessions, review code together, and pair program — all within LocalCode.</p>
      </div>
    </>
  );
}

export default function SettingsPanel() {
  const [activeSection, setActiveSection] = useState<SettingsSection>('general');

  return (
    <div style={styles.container}>
      <div style={styles.sidebar}>
        <div style={styles.sidebarHeader}>Settings</div>
        {SECTIONS.map((section) => (
          <div
            key={section.id}
            style={{
              ...styles.navItem,
              ...(activeSection === section.id ? styles.navItemActive : {}),
            }}
            onClick={() => setActiveSection(section.id)}
          >
            {section.icon}
            {section.label}
          </div>
        ))}
      </div>

      <div style={styles.content}>
        {activeSection === 'general' && <GeneralSettings />}
        {activeSection === 'editor' && <EditorSettings />}
        {activeSection === 'llm' && <LLMSettings />}
        {activeSection === 'agent' && <AgentSettings />}
        {activeSection === 'keybindings' && <KeybindingsSettings />}
        {activeSection === 'mcp' && <MCPSettingsSection />}
        {activeSection === 'remote' && <RemoteDevSettings />}
        {activeSection === 'collaboration' && <CollaborationSettings />}
      </div>
    </div>
  );
}
