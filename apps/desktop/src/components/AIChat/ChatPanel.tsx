import { useState, useRef, useEffect, useCallback } from 'react';
import { useAppStore } from '../../stores/appStore';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AgentStep, FileEntry } from '../../types';
import MentionPopup, { type MentionOption } from './MentionPopup';

const LANG_MAP: Record<string, string> = {
  ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact',
  py: 'python', rs: 'rust', go: 'go', java: 'java', c: 'c', cpp: 'cpp', h: 'c',
  html: 'html', css: 'css', scss: 'scss', json: 'json', md: 'markdown',
  yml: 'yaml', yaml: 'yaml', toml: 'toml', sh: 'shell', sql: 'sql',
  xml: 'xml', svg: 'xml', graphql: 'graphql', swift: 'swift', kt: 'kotlin',
  rb: 'ruby', php: 'php', lua: 'lua', zig: 'zig', svelte: 'svelte', vue: 'vue',
};

function getLang(path: string): string {
  return LANG_MAP[path.split('.').pop()?.toLowerCase() || ''] || 'plaintext';
}

/** Refresh the file tree from the current project path */
async function refreshFileTree() {
  const store = useAppStore.getState();
  if (!store.projectPath) return;
  try {
    const tree = await invoke<FileEntry[]>('read_dir', { path: store.projectPath });
    store.setFileTree(tree);
  } catch { /* ignore */ }
}

/** Open a file in the editor by its absolute path */
async function openFileInEditor(absPath: string) {
  try {
    const content = await invoke<string>('read_file', { path: absPath });
    const name = absPath.split('/').pop() || absPath;
    useAppStore.getState().openFile({
      path: absPath,
      name,
      content,
      language: getLang(absPath),
      modified: false,
    });
  } catch { /* file may not exist yet */ }
}

function AgentStepView({ step }: { step: AgentStep }) {
  return (
    <div className="agent-step">
      <div className="step-type">
        {step.type === 'tool_call' ? `Tool: ${step.tool}` : step.type}
      </div>
      <div className="step-content">
        {step.type === 'tool_call' && step.args && (
          <pre style={{ margin: 0, fontSize: 11 }}>{JSON.stringify(step.args, null, 2)}</pre>
        )}
        {step.type === 'tool_result' && step.result && (
          <pre style={{ margin: 0, fontSize: 11, maxHeight: 100, overflow: 'auto' }}>{step.result}</pre>
        )}
        {step.content && <span>{step.content}</span>}
      </div>
    </div>
  );
}

export default function ChatPanel() {
  const {
    chatMessages, addChatMessage, updateChatMessage, clearChat,
    isAIStreaming, setAIStreaming, agentMode, toggleAgentMode,
    openFiles, activeFile, llmConnected, projectPath,
  } = useAppStore();

  const [input, setInput] = useState('');
  const [mentionVisible, setMentionVisible] = useState(false);
  const [mentionFilter, setMentionFilter] = useState('');
  const [mentionPosition, setMentionPosition] = useState({ top: 0, left: 0 });
  const [mentionContext, setMentionContext] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatMessages]);

  // Listen for streaming responses
  useEffect(() => {
    const unlistenChat = listen<{ id: string; chunk: string }>('llm-chat-chunk', (event) => {
      const { id, chunk } = event.payload;
      const state = useAppStore.getState();
      const msg = state.chatMessages.find((m) => m.id === id);
      if (msg) {
        updateChatMessage(id, { content: msg.content + chunk });
      }
    });

    const unlistenDone = listen<{ id: string }>('llm-chat-done', () => {
      setAIStreaming(false);
    });

    const unlistenAgent = listen<{ id: string; step: AgentStep }>('agent-step', (event) => {
      const { id, step } = event.payload;
      const state = useAppStore.getState();
      const msg = state.chatMessages.find((m) => m.id === id);
      if (msg) {
        updateChatMessage(id, {
          agentSteps: [...(msg.agentSteps || []), step],
        });
      }

      // Handle open_in_editor tool results
      if (step.type === 'tool_result' && step.tool === 'open_in_editor' && step.result) {
        const match = step.result.match(/^OPEN_IN_EDITOR:(.+)$/);
        if (match) {
          const folderPath = match[1];
          const store = useAppStore.getState();
          store.setProjectPath(folderPath);
          store.setSidebarView('explorer');
          if (store.sidebarWidth === 0) store.setSidebarWidth(260);
          invoke<FileEntry[]>('read_dir', { path: folderPath })
            .then((tree) => {
              useAppStore.getState().setFileTree(tree);
            })
            .catch((err) => console.error('Failed to open folder:', err));
        }
      }

      // Auto-refresh file tree and open files when agent writes/creates/edits files
      if (step.type === 'tool_result' && step.result && !step.result.startsWith('Error')) {
        const tool = step.tool || '';
        if (tool === 'write_file' || tool === 'create_file' || tool === 'edit_file' || tool === 'delete_file') {
          // Refresh file tree
          refreshFileTree();

          // Auto-open the file in editor (extract path from result)
          if (tool === 'write_file' || tool === 'edit_file') {
            const pathMatch = step.result.match(/(?:wrote to|edited) (.+)$/i);
            if (pathMatch) {
              openFileInEditor(pathMatch[1]);
            }
          } else if (tool === 'create_file') {
            const pathMatch = step.result.match(/^Created (.+)$/i);
            if (pathMatch) {
              openFileInEditor(pathMatch[1]);
            }
          }
        }
      }
    });

    return () => {
      unlistenChat.then((fn) => fn());
      unlistenDone.then((fn) => fn());
      unlistenAgent.then((fn) => fn());
    };
  }, [updateChatMessage, setAIStreaming]);

  const handleMentionSelect = useCallback(async (option: MentionOption) => {
    setMentionVisible(false);
    const store = useAppStore.getState();
    let contextStr = '';

    switch (option.id) {
      case 'file': {
        // Inject active file content
        const af = store.activeFile;
        const fileData = af ? store.openFiles.find((f) => f.path === af) : null;
        if (fileData) {
          contextStr = `[File: ${fileData.name}]\n\`\`\`\n${fileData.content.slice(0, 6000)}\n\`\`\``;
        }
        break;
      }
      case 'codebase': {
        // Inject project file tree summary
        if (store.projectPath) {
          try {
            const tree = await invoke<FileEntry[]>('read_dir', { path: store.projectPath });
            const summarize = (entries: FileEntry[], depth = 0): string => {
              return entries.slice(0, 50).map((e) => {
                const indent = '  '.repeat(depth);
                const prefix = e.is_dir ? '/' : '';
                const children = e.children && e.is_dir ? '\n' + summarize(e.children, depth + 1) : '';
                return `${indent}${e.name}${prefix}${children}`;
              }).join('\n');
            };
            contextStr = `[Codebase Structure]\n${summarize(tree)}`;
          } catch { /* ignore */ }
        }
        break;
      }
      case 'git': {
        if (store.projectPath) {
          try {
            const status = await invoke<any[]>('git_status', { path: store.projectPath });
            const diff = await invoke<string>('git_diff', { path: store.projectPath });
            contextStr = `[Git Status]\n${JSON.stringify(status, null, 2)}\n\n[Git Diff]\n${(diff || '').slice(0, 4000)}`;
          } catch { /* ignore */ }
        }
        break;
      }
      case 'terminal': {
        const termOut = store.lastTerminalOutput;
        contextStr = termOut ? `[Terminal Output]\n\`\`\`\n${termOut.slice(0, 4000)}\n\`\`\`` : '[Terminal: no recent output]';
        break;
      }
      case 'selection': {
        const sel = store.editorSelection;
        contextStr = sel ? `[Editor Selection]\n\`\`\`\n${sel.slice(0, 4000)}\n\`\`\`` : '[No text selected in editor]';
        break;
      }
      case 'docs':
        contextStr = '[Docs: documentation context not yet available]';
        break;
    }

    setMentionContext((prev) => prev ? `${prev}\n\n${contextStr}` : contextStr);
    // Remove the @... from input
    const atIndex = input.lastIndexOf('@');
    if (atIndex >= 0) {
      setInput(input.slice(0, atIndex) + `${option.prefix} `);
    }
  }, [input]);

  const sendMessage = useCallback(async () => {
    const trimmed = input.trim();
    if (!trimmed || isAIStreaming) return;

    const userMsg = {
      id: `user-${Date.now()}`,
      role: 'user' as const,
      content: trimmed,
      timestamp: Date.now(),
    };
    addChatMessage(userMsg);
    setInput('');

    const assistantId = `assistant-${Date.now()}`;
    addChatMessage({
      id: assistantId,
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
      agentSteps: agentMode ? [] : undefined,
    });

    setAIStreaming(true);

    // Build context — combine mention context + current file
    const currentFileContent = activeFile
      ? openFiles.find((f) => f.path === activeFile)?.content || ''
      : '';

    const fullContext = [
      mentionContext,
      currentFileContent
        ? `Current file (${activeFile}):\n\`\`\`\n${currentFileContent.slice(0, 4000)}\n\`\`\``
        : '',
    ].filter(Boolean).join('\n\n');

    // Clear mention context after sending
    setMentionContext('');

    try {
      if (agentMode) {
        const history = chatMessages
          .filter((m) => m.content.trim())
          .map((m) => ({
            role: m.role,
            content: m.content.slice(0, 2000),
          }));

        await invoke('agent_execute', {
          responseId: assistantId,
          task: fullContext ? `${trimmed}\n\nContext:\n${fullContext}` : trimmed,
          projectPath: projectPath || '',
          currentFile: activeFile || '',
          currentFileContent,
          chatHistory: history,
        });
      } else {
        await invoke('llm_chat', {
          responseId: assistantId,
          messages: [
            ...chatMessages.map((m) => ({
              role: m.role,
              content: m.content,
            })),
            { role: 'user', content: trimmed },
          ],
          context: fullContext,
        });
      }
    } catch (err) {
      updateChatMessage(assistantId, {
        content: `Error: ${err}. Make sure a model is loaded.`,
      });
      setAIStreaming(false);
    }
  }, [input, isAIStreaming, agentMode, chatMessages, activeFile, openFiles, projectPath, addChatMessage, updateChatMessage, setAIStreaming, mentionContext]);

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);

    // Detect @ for mention popup
    const atIndex = val.lastIndexOf('@');
    if (atIndex >= 0 && (atIndex === 0 || val[atIndex - 1] === ' ' || val[atIndex - 1] === '\n')) {
      const afterAt = val.slice(atIndex + 1);
      // Only show if no space after the filter text (still typing mention)
      if (!afterAt.includes(' ')) {
        setMentionFilter(afterAt);
        setMentionVisible(true);
        // Position above textarea
        const rect = textareaRef.current?.getBoundingClientRect();
        if (rect) {
          setMentionPosition({ top: -250, left: 0 });
        }
        return;
      }
    }
    setMentionVisible(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (mentionVisible) return; // Let MentionPopup handle keys
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  return (
    <div className="chat-panel">
      <div className="model-selector">
        <span className={`model-dot ${llmConnected ? 'connected' : 'disconnected'}`} />
        <span style={{ color: 'var(--text-secondary)' }}>
          {llmConnected ? 'Model connected' : 'No model loaded'}
        </span>
        <button
          className="action-btn"
          style={{ marginLeft: 'auto' }}
          title="Load Model"
          onClick={async () => {
            try {
              const { open } = await import('@tauri-apps/plugin-dialog');
              const selected = await open({
                filters: [{ name: 'GGUF Models', extensions: ['gguf'] }],
              });
              if (selected) {
                await invoke('start_llm_server', { modelPath: selected as string });
              }
            } catch (err) {
              console.error('Failed to load model:', err);
            }
          }}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M11.7 8l-1-1-.7.7 1.6 1.6 3.1-3-.7-.7-2.3 2.4zM1 4h14V3H1v1zm9 3H1v1h9V7zm-4 3H1v1h5v-1z" />
          </svg>
        </button>
      </div>

      <div className="agent-toggle">
        <label>
          <input type="checkbox" checked={agentMode} onChange={toggleAgentMode} />
          Agent Mode
          {agentMode && <span className="badge">ON</span>}
        </label>
        <span style={{ marginLeft: 'auto', cursor: 'pointer', color: 'var(--text-muted)' }} onClick={clearChat}>
          Clear
        </span>
      </div>

      <div className="chat-messages">
        {chatMessages.length === 0 && (
          <div style={{ textAlign: 'center', color: 'var(--text-muted)', padding: 20 }}>
            <p style={{ marginBottom: 8 }}>AI Assistant</p>
            <p style={{ fontSize: 11 }}>Ask questions about your code, get completions, or enable Agent Mode for autonomous tasks.</p>
          </div>
        )}
        {chatMessages.map((msg) => (
          <div key={msg.id} className={`chat-message ${msg.role}`}>
            <span className="role">{msg.role}</span>
            <div className="body">
              {msg.agentSteps?.map((step, i) => (
                <AgentStepView key={i} step={step} />
              ))}
              {msg.content || (isAIStreaming && msg.role === 'assistant' ? '...' : '')}
            </div>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>

      <div className="chat-input-area" style={{ position: 'relative' }}>
        <MentionPopup
          visible={mentionVisible}
          filter={mentionFilter}
          position={mentionPosition}
          onSelect={handleMentionSelect}
          onClose={() => setMentionVisible(false)}
        />
        {mentionContext && (
          <div style={{ padding: '4px 8px', fontSize: 10, color: '#4ec9b0', background: 'rgba(78,201,176,0.1)', borderRadius: 3, marginBottom: 4 }}>
            Context attached (type @ to add more)
          </div>
        )}
        <textarea
          ref={textareaRef}
          value={input}
          onChange={handleInputChange}
          onKeyDown={handleKeyDown}
          placeholder={agentMode ? 'Describe a task... (Agent will execute it) — type @ for context' : 'Ask about your code... — type @ for context'}
          rows={1}
          disabled={isAIStreaming}
        />
        <button onClick={sendMessage} disabled={isAIStreaming || !input.trim()}>
          {isAIStreaming ? '...' : 'Send'}
        </button>
      </div>
    </div>
  );
}
