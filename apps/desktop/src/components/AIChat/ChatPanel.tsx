import { useState, useRef, useEffect, useCallback } from 'react';
import { useAppStore } from '../../stores/appStore';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AgentStep, FileEntry, ChatSessionInfo, ChatSearchResult } from '../../types';
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

/** Track original file contents before agent writes (for diff review) */
const pendingOriginals: Record<string, string> = {};

// Friendly messages for different agent actions
function getStepEmoji(step: AgentStep): string {
  if (step.type === 'thinking') {
    if (step.content?.startsWith('Plan')) return '📋';
    if (step.content?.includes('Step')) return '⚡';
    return '🤔';
  }
  if (step.type === 'tool_call') {
    if (step.tool === 'write_file') return '✍️';
    if (step.tool === 'run_command') return '🚀';
    if (step.tool === 'read_file') return '📖';
    if (step.tool === 'edit_file') return '🔧';
    if (step.tool === 'list_dir') return '📂';
    return '🔨';
  }
  if (step.type === 'tool_result') {
    if (step.result?.includes('Error') || step.result?.includes('ERROR')) return '⚠️';
    return '✅';
  }
  return '💬';
}

function getFriendlyLabel(step: AgentStep): string {
  if (step.type === 'thinking') {
    if (step.content?.startsWith('Planning')) return 'Planning your project...';
    if (step.content?.startsWith('Plan:')) return 'Here\'s the game plan';
    if (step.content?.includes('Step')) return step.content || 'Working on it...';
    if (step.content?.includes('failed')) return 'Switching to direct mode...';
    return step.content || 'Thinking...';
  }
  if (step.type === 'tool_call') {
    const args = step.args as Record<string, unknown> | undefined;
    if (step.tool === 'write_file') {
      const path = (args?.path || args?.file_path || 'file') as string;
      return `Writing ${path}...`;
    }
    if (step.tool === 'run_command') {
      const cmd = (args?.command || '') as string;
      if (cmd.includes('pip') || cmd.includes('npm')) return `Installing packages...`;
      if (cmd.includes('python') || cmd.includes('node')) return `Running your code...`;
      return `Running command...`;
    }
    if (step.tool === 'read_file') return 'Reading file...';
    if (step.tool === 'edit_file') return 'Fixing code...';
    if (step.tool === 'list_dir') return 'Looking around...';
    return `Using ${step.tool}...`;
  }
  if (step.type === 'tool_result') {
    if (step.result?.includes('Successfully wrote')) return 'File created!';
    if (step.result?.includes('Error') || step.result?.includes('ERROR')) return 'Hit a snag, fixing it...';
    return 'Done!';
  }
  return step.type;
}

function AgentStepView({ step }: { step: AgentStep }) {
  const [expanded, setExpanded] = useState(false);
  const emoji = getStepEmoji(step);
  const label = getFriendlyLabel(step);
  const hasDetails = (step.type === 'tool_call' && step.args) ||
    (step.type === 'tool_result' && step.result) ||
    (step.type === 'thinking' && step.content && step.content.includes('\n'));

  return (
    <div className="agent-step">
      <div
        className={`step-header ${hasDetails ? 'clickable' : ''}`}
        onClick={() => hasDetails && setExpanded(!expanded)}
      >
        <span className="step-emoji">{emoji}</span>
        <span className="step-label">{label}</span>
        {hasDetails && <span className="step-toggle">{expanded ? '▾' : '▸'}</span>}
      </div>
      {expanded && (
        <div className="step-details">
          {step.type === 'tool_call' && step.args && (
            <pre>{JSON.stringify(step.args, null, 2)}</pre>
          )}
          {step.type === 'tool_result' && step.result && (
            <pre>{step.result}</pre>
          )}
          {step.type === 'thinking' && step.content && (
            <pre>{step.content}</pre>
          )}
        </div>
      )}
    </div>
  );
}

const PROVIDER_OPTIONS = [
  { value: 'local', label: 'Local Model' },
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'ollama', label: 'Ollama' },
];

export default function ChatPanel() {
  const {
    chatMessages, addChatMessage, updateChatMessage, clearChat,
    isAIStreaming, setAIStreaming, agentMode, toggleAgentMode,
    openFiles, activeFile, llmConnected, projectPath,
    selectedProvider, setSelectedProvider,
    agentPlanMode, setAgentPlanMode, agentPlan, setAgentPlan,
    createCheckpoint, restoreCheckpoint, checkpoints,
    activeChatSessionId, setActiveChatSessionId,
    chatSessions, setChatSessions,
  } = useAppStore();

  const [input, setInput] = useState('');
  const [mentionVisible, setMentionVisible] = useState(false);
  const [mentionFilter, setMentionFilter] = useState('');
  const [mentionPosition, setMentionPosition] = useState({ top: 0, left: 0 });
  const [mentionContext, setMentionContext] = useState('');
  const [isRecording, setIsRecording] = useState(false);
  const [chatImages, setChatImages] = useState<string[]>([]);
  const [slashCommands, setSlashCommands] = useState<string[]>([]);
  const [showSlashPopup, setShowSlashPopup] = useState(false);
  const [showSessionSwitcher, setShowSessionSwitcher] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<ChatSearchResult[]>([]);
  const [showSearch, setShowSearch] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const recognitionRef = useRef<any>(null);

  // ── Chat Persistence: Initialize session on mount / project change ──
  useEffect(() => {
    const initSession = async () => {
      try {
        const sessions = await invoke<ChatSessionInfo[]>('chat_list_sessions', {
          projectPath: projectPath || undefined,
          limit: 50,
        });
        setChatSessions(sessions);

        if (sessions.length > 0) {
          // Load most recent session
          const latest = sessions[0];
          setActiveChatSessionId(latest.id);
          const messages = await invoke<{ id: string; chat_session_id: string; role: string; content: string; timestamp: number; agent_steps: string | null }[]>(
            'chat_get_messages',
            { sessionId: latest.id }
          );
          // Load persisted messages into Zustand
          clearChat();
          for (const msg of messages) {
            addChatMessage({
              id: msg.id,
              role: msg.role as 'user' | 'assistant' | 'system',
              content: msg.content,
              timestamp: msg.timestamp,
              agentSteps: msg.agent_steps ? JSON.parse(msg.agent_steps) : undefined,
            });
          }
        } else {
          // Create new session
          const session = await invoke<ChatSessionInfo>('chat_create_session', {
            projectPath: projectPath || '',
            title: 'New Chat',
          });
          setActiveChatSessionId(session.id);
          setChatSessions([session]);
          clearChat();
        }
      } catch (err) {
        console.error('Failed to init chat session:', err);
      }
    };
    initSession();
  }, [projectPath]);

  // ── Persist messages fire-and-forget ──
  const persistMessage = useCallback((msg: { id: string; role: string; content: string; timestamp: number; agentSteps?: AgentStep[] }) => {
    if (!activeChatSessionId) return;
    invoke('chat_add_message', {
      id: msg.id,
      chatSessionId: activeChatSessionId,
      role: msg.role,
      content: msg.content,
      timestamp: msg.timestamp,
      agentSteps: msg.agentSteps ? JSON.stringify(msg.agentSteps) : null,
    }).catch(console.error);
  }, [activeChatSessionId]);

  const persistMessageUpdate = useCallback((id: string, content: string, agentSteps?: AgentStep[]) => {
    invoke('chat_update_message', {
      id,
      content,
      agentSteps: agentSteps ? JSON.stringify(agentSteps) : null,
    }).catch(console.error);
  }, []);

  // ── Persist streaming completion ──
  useEffect(() => {
    // When streaming ends, persist the final assistant message
    if (!isAIStreaming) {
      const lastMsg = chatMessages[chatMessages.length - 1];
      if (lastMsg && lastMsg.role === 'assistant' && lastMsg.content) {
        persistMessageUpdate(lastMsg.id, lastMsg.content, lastMsg.agentSteps);
      }
    }
  }, [isAIStreaming]);

  // ── Switch session ──
  const switchSession = useCallback(async (sessionId: string) => {
    try {
      const messages = await invoke<{ id: string; chat_session_id: string; role: string; content: string; timestamp: number; agent_steps: string | null }[]>(
        'chat_get_messages',
        { sessionId }
      );
      clearChat();
      for (const msg of messages) {
        addChatMessage({
          id: msg.id,
          role: msg.role as 'user' | 'assistant' | 'system',
          content: msg.content,
          timestamp: msg.timestamp,
          agentSteps: msg.agent_steps ? JSON.parse(msg.agent_steps) : undefined,
        });
      }
      setActiveChatSessionId(sessionId);
      setShowSessionSwitcher(false);
    } catch (err) {
      console.error('Failed to switch session:', err);
    }
  }, [clearChat, addChatMessage, setActiveChatSessionId]);

  // ── New chat (keeps old session in DB) ──
  const startNewChat = useCallback(async () => {
    try {
      const session = await invoke<ChatSessionInfo>('chat_create_session', {
        projectPath: projectPath || '',
        title: 'New Chat',
      });
      setActiveChatSessionId(session.id);
      clearChat();
      // Refresh session list
      const sessions = await invoke<ChatSessionInfo[]>('chat_list_sessions', {
        projectPath: projectPath || undefined,
        limit: 50,
      });
      setChatSessions(sessions);
    } catch (err) {
      console.error('Failed to start new chat:', err);
    }
  }, [projectPath, clearChat, setActiveChatSessionId, setChatSessions]);

  // ── Search chat history ──
  const handleSearch = useCallback(async (query: string) => {
    setSearchQuery(query);
    if (!query.trim()) {
      setSearchResults([]);
      return;
    }
    try {
      const results = await invoke<ChatSearchResult[]>('chat_search', {
        query,
        projectPath: projectPath || undefined,
        topK: 10,
      });
      setSearchResults(results);
    } catch (err) {
      console.error('Chat search failed:', err);
    }
  }, [projectPath]);

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

      // Capture original file content before agent writes (for diff review)
      if (step.type === 'tool_call' && step.args) {
        const tool = step.tool || '';
        if (tool === 'write_file' || tool === 'edit_file') {
          const filePath = (step.args as any).path;
          if (filePath && !pendingOriginals[filePath]) {
            invoke<string>('read_file', { path: filePath })
              .then((content) => { pendingOriginals[filePath] = content; })
              .catch(() => { pendingOriginals[filePath] = ''; });
          }
        }
      }

      // Auto-refresh file tree and open files when agent writes/creates/edits files
      if (step.type === 'tool_result' && step.result && !step.result.startsWith('Error')) {
        const tool = step.tool || '';
        if (tool === 'write_file' || tool === 'create_file' || tool === 'edit_file' || tool === 'delete_file') {
          // Refresh file tree
          refreshFileTree();

          // Auto-open the file in editor (extract path from result)
          let filePath: string | null = null;
          if (tool === 'write_file' || tool === 'edit_file') {
            const pathMatch = step.result.match(/(?:wrote to|edited) (.+)$/i);
            if (pathMatch) filePath = pathMatch[1];
          } else if (tool === 'create_file') {
            const pathMatch = step.result.match(/^Created (.+)$/i);
            if (pathMatch) filePath = pathMatch[1];
          }

          if (filePath) {
            openFileInEditor(filePath);

            // Store pending agent change for diff review
            if (tool === 'write_file' || tool === 'edit_file') {
              const original = pendingOriginals[filePath] || '';
              invoke<string>('read_file', { path: filePath })
                .then((modified) => {
                  if (original !== modified) {
                    useAppStore.getState().addPendingAgentChange(filePath!, original, modified);
                  }
                })
                .catch(() => {});
              delete pendingOriginals[filePath];
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
        const af = store.activeFile;
        const fileData = af ? store.openFiles.find((f) => f.path === af) : null;
        if (fileData) {
          contextStr = `[File: ${fileData.name}]\n\`\`\`\n${fileData.content.slice(0, 6000)}\n\`\`\``;
        }
        break;
      }
      case 'codebase': {
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
      case 'web': {
        contextStr = '[Web Search: will search the web for relevant results]';
        break;
      }
      case 'docs':
        contextStr = '[Docs: documentation context not yet available]';
        break;
    }

    setMentionContext((prev) => prev ? `${prev}\n\n${contextStr}` : contextStr);
    const atIndex = input.lastIndexOf('@');
    if (atIndex >= 0) {
      setInput(input.slice(0, atIndex) + `${option.prefix} `);
    }
  }, [input]);

  const sendMessage = useCallback(async () => {
    const trimmed = input.trim();
    if (!trimmed || isAIStreaming) return;

    // Handle slash commands (Feature 10)
    if (trimmed.startsWith('/') && projectPath) {
      const cmdName = trimmed.slice(1).split(' ')[0];
      const cmdPath = `${projectPath}/.localcode/commands/${cmdName}.md`;
      try {
        const cmdContent = await invoke<string>('read_file', { path: cmdPath });
        setMentionContext((prev) => prev ? `${prev}\n\n[Slash Command: /${cmdName}]\n${cmdContent}` : `[Slash Command: /${cmdName}]\n${cmdContent}`);
        const restOfMessage = trimmed.slice(cmdName.length + 1).trim();
        if (restOfMessage) {
          setInput(restOfMessage);
        } else {
          setInput('');
        }
        // Don't send yet — let user add more context
        return;
      } catch {
        // Not a valid slash command, proceed normally
      }
    }

    const userMsg = {
      id: `user-${Date.now()}`,
      role: 'user' as const,
      content: trimmed,
      timestamp: Date.now(),
    };
    addChatMessage(userMsg);
    persistMessage(userMsg);
    setInput('');
    setChatImages([]);

    const assistantId = `assistant-${Date.now()}`;
    const assistantMsg = {
      id: assistantId,
      role: 'assistant' as const,
      content: '',
      timestamp: Date.now(),
      agentSteps: agentMode ? [] : undefined,
    };
    addChatMessage(assistantMsg);
    persistMessage(assistantMsg);

    setAIStreaming(true);

    // Build context
    const currentFileContent = activeFile
      ? openFiles.find((f) => f.path === activeFile)?.content || ''
      : '';

    let codebaseContext = '';
    if (projectPath) {
      try {
        const chunks = await invoke<string[]>('search_codebase', {
          projectPath,
          query: trimmed,
          topK: 5,
        });
        if (chunks && chunks.length > 0) {
          codebaseContext = `## Relevant Code Context\n${chunks.join('\n\n')}`;
        }
      } catch {
        // Silent skip
      }
    }

    const imageContext = chatImages.length > 0
      ? `\n\n[${chatImages.length} image(s) attached]`
      : '';

    const fullContext = [
      codebaseContext,
      mentionContext,
      currentFileContent
        ? `Current file (${activeFile}):\n\`\`\`\n${currentFileContent.slice(0, 4000)}\n\`\`\``
        : '',
      imageContext,
    ].filter(Boolean).join('\n\n');

    setMentionContext('');

    const providerName = selectedProvider !== 'local' ? selectedProvider : undefined;

    try {
      if (agentMode) {
        // Feature 11: Create checkpoint before agent execution
        createCheckpoint();

        // Feature 7: Agent Plan Mode
        // If a plan already exists and the user is asking to execute, run the agent directly
        const isExecuteRequest = agentPlan && /^(execute|run|do it|go ahead|yes|proceed|ok|execute it|start)/i.test(trimmed);

        if (agentPlanMode && !isExecuteRequest) {
          // Generate a plan first via llm_chat
          // Reuse the assistantId for the plan so the Execute/Cancel buttons show on it
          await invoke('llm_chat', {
            responseId: assistantId,
            messages: [{ role: 'user', content: trimmed }],
            context: `You are a planning assistant. Create a step-by-step plan for this task. Do NOT execute anything. Just list the numbered steps.\n\n${fullContext}`,
            providerName,
          });

          // The plan will be streamed via llm-chat-chunk events
          // Set the plan in store so the UI can show Execute/Cancel buttons
          setAgentPlan(assistantId);
          return;
        }

        // If user typed execute request, clear the plan and proceed with agent_execute
        if (isExecuteRequest) {
          setAgentPlan(null);
        }

        const history = chatMessages
          .filter((m) => m.content.trim())
          .map((m) => ({
            role: m.role,
            content: m.content.slice(0, 2000),
          }));

        // Find the original task (last substantial user message before "execute it")
        const isLocal = selectedProvider === 'local' || !selectedProvider;
        const taskMessage = isExecuteRequest
          ? chatMessages.filter((m) => m.role === 'user' && m.content.trim().length > 20).pop()?.content || trimmed
          : isLocal
            ? trimmed  // Local models: just the raw task to save tokens
            : (fullContext ? `${trimmed}\n\nContext:\n${fullContext}` : trimmed);

        await invoke('agent_execute', {
          responseId: assistantId,
          task: taskMessage,
          projectPath: projectPath || '',
          currentFile: activeFile || '',
          currentFileContent: isLocal ? '' : currentFileContent,  // Skip for local models
          chatHistory: isLocal ? [] : history,  // Skip for local models
          providerName,
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
          providerName,
          images: chatImages.length > 0 ? chatImages : undefined,
        });
      }
    } catch (err) {
      updateChatMessage(assistantId, {
        content: `Error: ${err}. Make sure a model is loaded.`,
      });
      setAIStreaming(false);
    }
  }, [input, isAIStreaming, agentMode, agentPlanMode, agentPlan, chatMessages, activeFile, openFiles, projectPath, addChatMessage, updateChatMessage, setAIStreaming, mentionContext, selectedProvider, chatImages, createCheckpoint, setAgentPlan, persistMessage]);

  // Execute plan (Feature 7)
  const executePlan = useCallback(async () => {
    if (!agentPlan) return;
    setAgentPlan(null);

    const assistantId = `agent-${Date.now()}`;
    addChatMessage({
      id: assistantId,
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
      agentSteps: [],
    });

    setAIStreaming(true);

    const lastUserMsg = chatMessages.filter((m) => m.role === 'user').pop();
    const providerName = selectedProvider !== 'local' ? selectedProvider : undefined;
    const currentFileContent = activeFile
      ? openFiles.find((f) => f.path === activeFile)?.content || ''
      : '';

    try {
      const history = chatMessages
        .filter((m) => m.content.trim())
        .map((m) => ({ role: m.role, content: m.content.slice(0, 2000) }));

      await invoke('agent_execute', {
        responseId: assistantId,
        task: lastUserMsg?.content || '',
        projectPath: projectPath || '',
        currentFile: activeFile || '',
        currentFileContent,
        chatHistory: history,
        providerName,
      });
    } catch (err) {
      updateChatMessage(assistantId, {
        content: `Error: ${err}`,
      });
      setAIStreaming(false);
    }
  }, [agentPlan, chatMessages, selectedProvider, activeFile, openFiles, projectPath, addChatMessage, updateChatMessage, setAIStreaming, setAgentPlan]);

  // Voice input (Feature 18)
  const toggleVoiceInput = useCallback(() => {
    if (isRecording) {
      recognitionRef.current?.stop();
      setIsRecording(false);
      return;
    }

    const SpeechRecognition = (window as any).webkitSpeechRecognition || (window as any).SpeechRecognition;
    if (!SpeechRecognition) {
      console.warn('Speech recognition not supported');
      return;
    }

    const recognition = new SpeechRecognition();
    recognition.continuous = true;
    recognition.interimResults = true;
    recognition.lang = 'en-US';

    recognition.onresult = (event: any) => {
      let transcript = '';
      for (let i = event.resultIndex; i < event.results.length; i++) {
        transcript += event.results[i][0].transcript;
      }
      if (event.results[event.results.length - 1].isFinal) {
        setInput((prev) => prev + transcript + ' ');
      }
    };

    recognition.onerror = () => setIsRecording(false);
    recognition.onend = () => setIsRecording(false);

    recognitionRef.current = recognition;
    recognition.start();
    setIsRecording(true);
  }, [isRecording]);

  // Image paste/drop (Feature 14)
  const handlePaste = useCallback((e: React.ClipboardEvent) => {
    const items = e.clipboardData.items;
    for (let i = 0; i < items.length; i++) {
      if (items[i].type.startsWith('image/')) {
        e.preventDefault();
        const file = items[i].getAsFile();
        if (!file) continue;
        const reader = new FileReader();
        reader.onload = () => {
          const base64 = reader.result as string;
          setChatImages((prev) => [...prev, base64]);
        };
        reader.readAsDataURL(file);
        break;
      }
    }
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const files = e.dataTransfer.files;
    for (let i = 0; i < files.length; i++) {
      if (files[i].type.startsWith('image/')) {
        const reader = new FileReader();
        reader.onload = () => {
          const base64 = reader.result as string;
          setChatImages((prev) => [...prev, base64]);
        };
        reader.readAsDataURL(files[i]);
      }
    }
  }, []);

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInput(val);

    // Detect / at start for slash commands (Feature 10)
    if (val.startsWith('/') && projectPath) {
      const filter = val.slice(1).split(' ')[0];
      loadSlashCommands(filter);
      return;
    }
    setShowSlashPopup(false);

    // Detect @ for mention popup
    const atIndex = val.lastIndexOf('@');
    if (atIndex >= 0 && (atIndex === 0 || val[atIndex - 1] === ' ' || val[atIndex - 1] === '\n')) {
      const afterAt = val.slice(atIndex + 1);
      if (!afterAt.includes(' ')) {
        setMentionFilter(afterAt);
        setMentionVisible(true);
        const rect = textareaRef.current?.getBoundingClientRect();
        if (rect) {
          setMentionPosition({ top: -250, left: 0 });
        }
        return;
      }
    }
    setMentionVisible(false);
  };

  const loadSlashCommands = useCallback(async (filter: string) => {
    if (!projectPath) return;
    try {
      const results = await invoke<{ path: string; name: string }[]>('search_files', {
        path: `${projectPath}/.localcode/commands`,
        query: '',
      });
      const cmds = results
        .filter((r) => r.name.endsWith('.md'))
        .map((r) => r.name.replace('.md', ''))
        .filter((name) => name.startsWith(filter));
      setSlashCommands(cmds);
      setShowSlashPopup(cmds.length > 0);
    } catch {
      setShowSlashPopup(false);
    }
  }, [projectPath]);

  const selectSlashCommand = useCallback((cmd: string) => {
    setInput(`/${cmd} `);
    setShowSlashPopup(false);
    textareaRef.current?.focus();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (mentionVisible || showSlashPopup) return;
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  // Restore checkpoint handler
  const handleRestore = useCallback((cpId: string) => {
    restoreCheckpoint(cpId);
    // Also write restored files back to disk
    const cp = checkpoints.find((c) => c.id === cpId);
    if (cp) {
      Object.entries(cp.files).forEach(([path, content]) => {
        invoke('write_file', { path, content }).catch(() => {});
      });
    }
  }, [restoreCheckpoint, checkpoints]);

  return (
    <div className="chat-panel">
      <div className="model-selector">
        <span className={`model-dot ${llmConnected ? 'connected' : 'disconnected'}`} />
        <select
          value={selectedProvider}
          onChange={(e) => setSelectedProvider(e.target.value)}
          title="Select AI Provider"
        >
          {PROVIDER_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>{opt.label}</option>
          ))}
        </select>
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

      {/* Session switcher */}
      <div style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: 4, padding: '4px 8px', borderBottom: '1px solid var(--border-color)', fontSize: 11 }}>
        <span
          style={{ cursor: 'pointer', flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', color: 'var(--text-secondary)' }}
          onClick={() => setShowSessionSwitcher(!showSessionSwitcher)}
          title="Switch chat session"
        >
          {chatSessions.find((s) => s.id === activeChatSessionId)?.title || 'New Chat'} ▾
        </span>
        <span
          style={{ cursor: 'pointer', color: 'var(--text-muted)', fontSize: 12 }}
          onClick={() => setShowSearch(!showSearch)}
          title="Search chat history"
        >
          &#x1F50D;
        </span>
        <span
          style={{ cursor: 'pointer', color: 'var(--text-muted)', fontSize: 14 }}
          onClick={startNewChat}
          title="New chat"
        >
          +
        </span>
        {showSessionSwitcher && chatSessions.length > 0 && (
          <div style={{
            position: 'absolute', top: '100%', left: 0, right: 0, zIndex: 30,
            background: 'var(--bg-secondary)', border: '1px solid var(--border-color)',
            borderRadius: 4, boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
            maxHeight: 240, overflow: 'auto',
          }}>
            {chatSessions.map((s) => (
              <div
                key={s.id}
                onClick={() => switchSession(s.id)}
                style={{
                  padding: '6px 10px', cursor: 'pointer', fontSize: 11,
                  background: s.id === activeChatSessionId ? 'var(--bg-hover)' : 'transparent',
                  borderBottom: '1px solid var(--border-color)',
                }}
                onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-hover)'; }}
                onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = s.id === activeChatSessionId ? 'var(--bg-hover)' : 'transparent'; }}
              >
                <div style={{ color: 'var(--text-primary)', fontWeight: s.id === activeChatSessionId ? 600 : 400 }}>{s.title}</div>
                <div style={{ color: 'var(--text-muted)', fontSize: 10 }}>
                  {new Date(s.updated_at * 1000).toLocaleDateString()} · {s.message_count} msgs
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      {/* Search bar */}
      {showSearch && (
        <div style={{ padding: '4px 8px', borderBottom: '1px solid var(--border-color)' }}>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => handleSearch(e.target.value)}
            placeholder="Search chat history..."
            style={{
              width: '100%', background: 'var(--bg-primary)', border: '1px solid var(--border-color)',
              borderRadius: 3, padding: '3px 6px', fontSize: 11, color: 'var(--text-primary)',
              outline: 'none',
            }}
          />
          {searchResults.length > 0 && (
            <div style={{ maxHeight: 200, overflow: 'auto', marginTop: 4 }}>
              {searchResults.map((r) => (
                <div
                  key={r.message_id}
                  onClick={() => { switchSession(r.chat_session_id); setShowSearch(false); setSearchQuery(''); setSearchResults([]); }}
                  style={{ padding: '4px 6px', cursor: 'pointer', fontSize: 10, borderBottom: '1px solid var(--border-color)' }}
                  onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-hover)'; }}
                  onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
                >
                  <div style={{ color: 'var(--accent)', fontSize: 10 }}>{r.session_title}</div>
                  <div style={{ color: 'var(--text-secondary)' }}>{r.content.slice(0, 100)}...</div>
                  <div style={{ color: 'var(--text-muted)' }}>{r.role} · score: {r.score.toFixed(2)}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="agent-toggle">
        <label>
          <input type="checkbox" checked={agentMode} onChange={toggleAgentMode} />
          Agent Mode
          {agentMode && <span className="badge">ON</span>}
        </label>
        {agentMode && (
          <label style={{ marginLeft: 8 }}>
            <input
              type="checkbox"
              checked={agentPlanMode}
              onChange={(e) => setAgentPlanMode(e.target.checked)}
            />
            Plan First
          </label>
        )}
        <span style={{ marginLeft: 'auto', cursor: 'pointer', color: 'var(--text-muted)' }} onClick={startNewChat}>
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
            {/* Plan mode buttons (Feature 7) */}
            {agentPlan === msg.id && !isAIStreaming && msg.content && (
              <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
                <button
                  onClick={executePlan}
                  style={{
                    background: 'var(--accent-green)',
                    border: 'none',
                    borderRadius: 4,
                    color: '#1e1e1e',
                    padding: '4px 12px',
                    cursor: 'pointer',
                    fontSize: 12,
                    fontWeight: 600,
                  }}
                >
                  Execute Plan
                </button>
                <button
                  onClick={() => setAgentPlan(null)}
                  style={{
                    background: 'none',
                    border: '1px solid var(--border-color)',
                    borderRadius: 4,
                    color: 'var(--text-secondary)',
                    padding: '4px 12px',
                    cursor: 'pointer',
                    fontSize: 12,
                  }}
                >
                  Cancel
                </button>
              </div>
            )}
          </div>
        ))}
        {/* Checkpoint restore button (Feature 11) */}
        {checkpoints.length > 0 && !isAIStreaming && (
          <div style={{ padding: '4px 0' }}>
            {checkpoints.slice(0, 1).map((cp) => (
              <button
                key={cp.id}
                onClick={() => handleRestore(cp.id)}
                style={{
                  background: 'none',
                  border: '1px solid var(--accent-orange)',
                  borderRadius: 4,
                  color: 'var(--accent-orange)',
                  padding: '3px 10px',
                  cursor: 'pointer',
                  fontSize: 11,
                }}
              >
                Restore to checkpoint ({new Date(cp.timestamp).toLocaleTimeString()})
              </button>
            ))}
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      <div
        className="chat-input-area"
        style={{ position: 'relative', flexDirection: 'column' }}
        onDrop={handleDrop}
        onDragOver={(e) => e.preventDefault()}
      >
        <MentionPopup
          visible={mentionVisible}
          filter={mentionFilter}
          position={mentionPosition}
          onSelect={handleMentionSelect}
          onClose={() => setMentionVisible(false)}
        />
        {/* Slash command popup (Feature 10) */}
        {showSlashPopup && slashCommands.length > 0 && (
          <div style={{
            position: 'absolute',
            bottom: '100%',
            left: 0,
            right: 0,
            background: 'var(--bg-secondary)',
            border: '1px solid var(--border-color)',
            borderRadius: 4,
            boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
            maxHeight: 200,
            overflow: 'auto',
            zIndex: 20,
          }}>
            {slashCommands.map((cmd) => (
              <div
                key={cmd}
                onClick={() => selectSlashCommand(cmd)}
                style={{ padding: '6px 12px', cursor: 'pointer', fontSize: 12, color: 'var(--text-primary)' }}
                onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-hover)'; }}
                onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
              >
                <span style={{ color: 'var(--accent)' }}>/{cmd}</span>
              </div>
            ))}
          </div>
        )}
        {mentionContext && (
          <div style={{ padding: '4px 8px', fontSize: 10, color: '#4ec9b0', background: 'rgba(78,201,176,0.1)', borderRadius: 3, marginBottom: 4 }}>
            Context attached (type @ to add more)
          </div>
        )}
        {/* Image thumbnails (Feature 14) */}
        {chatImages.length > 0 && (
          <div style={{ display: 'flex', gap: 4, padding: '4px 0', flexWrap: 'wrap' }}>
            {chatImages.map((img, i) => (
              <div key={i} style={{ position: 'relative' }}>
                <img src={img} alt="" style={{ width: 48, height: 48, objectFit: 'cover', borderRadius: 4, border: '1px solid var(--border-color)' }} />
                <span
                  onClick={() => setChatImages((prev) => prev.filter((_, idx) => idx !== i))}
                  style={{
                    position: 'absolute', top: -4, right: -4,
                    width: 14, height: 14, borderRadius: '50%',
                    background: 'var(--accent-red)', color: '#fff',
                    fontSize: 10, display: 'flex', alignItems: 'center', justifyContent: 'center',
                    cursor: 'pointer',
                  }}
                >x</span>
              </div>
            ))}
          </div>
        )}
        <div style={{ display: 'flex', gap: 8, width: '100%' }}>
          <textarea
            ref={textareaRef}
            value={input}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            placeholder={agentMode ? 'Describe a task... — type @ for context, / for commands' : 'Ask about your code... — type @ for context'}
            rows={1}
            disabled={isAIStreaming}
          />
          {/* Voice input button (Feature 18) */}
          <button
            onClick={toggleVoiceInput}
            title={isRecording ? 'Stop Recording' : 'Voice Input'}
            style={{
              background: isRecording ? 'var(--accent-red)' : 'none',
              border: `1px solid ${isRecording ? 'var(--accent-red)' : 'var(--border-color)'}`,
              borderRadius: 4,
              color: isRecording ? '#fff' : 'var(--text-secondary)',
              padding: '0 8px',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 4,
              alignSelf: 'flex-end',
              height: 36,
            }}
          >
            {isRecording && <span className="voice-recording" />}
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 11a3 3 0 003-3V3a3 3 0 00-6 0v5a3 3 0 003 3zm5-3a5 5 0 01-4.5 4.975V15h-1v-2.025A5 5 0 013 8h1a4 4 0 008 0h1z" />
            </svg>
          </button>
          <button onClick={sendMessage} disabled={isAIStreaming || !input.trim()}>
            {isAIStreaming ? '...' : 'Send'}
          </button>
        </div>
      </div>
    </div>
  );
}
