import { useState, useRef, useEffect, useCallback } from 'react';
import { useAppStore } from '../../stores/appStore';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AgentStep } from '../../types';

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
    });

    return () => {
      unlistenChat.then((fn) => fn());
      unlistenDone.then((fn) => fn());
      unlistenAgent.then((fn) => fn());
    };
  }, [updateChatMessage, setAIStreaming]);

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

    // Build context
    const currentFileContent = activeFile
      ? openFiles.find((f) => f.path === activeFile)?.content || ''
      : '';

    try {
      if (agentMode) {
        // Build conversation history for agent memory
        const history = chatMessages
          .filter((m) => m.content.trim())
          .map((m) => ({
            role: m.role,
            content: m.content.slice(0, 2000), // trim long messages
          }));

        await invoke('agent_execute', {
          responseId: assistantId,
          task: trimmed,
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
          context: currentFileContent
            ? `Current file (${activeFile}):\n\`\`\`\n${currentFileContent.slice(0, 4000)}\n\`\`\``
            : '',
        });
      }
    } catch (err) {
      updateChatMessage(assistantId, {
        content: `Error: ${err}. Make sure a model is loaded.`,
      });
      setAIStreaming(false);
    }
  }, [input, isAIStreaming, agentMode, chatMessages, activeFile, openFiles, projectPath, addChatMessage, updateChatMessage, setAIStreaming]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
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

      <div className="chat-input-area">
        <textarea
          ref={textareaRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={agentMode ? 'Describe a task... (Agent will execute it)' : 'Ask about your code...'}
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
