import { describe, it, expect, beforeEach } from 'vitest';
import { useAppStore } from '../appStore';

// Reset store before each test
beforeEach(() => {
  useAppStore.setState({
    openFiles: [],
    activeFile: null,
    sidebarView: 'explorer',
    sidebarWidth: 260,
    terminalVisible: true,
    chatMessages: [],
    isAIStreaming: false,
    agentMode: false,
    chatPanelVisible: true,
    chatPanelWidth: 320,
    theme: 'dark',
    llmConnected: false,
    completionStatus: 'idle',
    editorSelection: '',
    lastTerminalOutput: '',
    quickOpenVisible: false,
    inlineEditVisible: false,
    gitStatus: [],
    currentBranch: '',
    problems: [],
    downloadProgress: {},
  });
});

describe('File Management', () => {
  it('opens a file and sets it as active', () => {
    const file = {
      path: '/test/file.ts',
      name: 'file.ts',
      content: 'const x = 1;',
      language: 'typescript',
      modified: false,
    };

    useAppStore.getState().openFile(file);

    const state = useAppStore.getState();
    expect(state.openFiles).toHaveLength(1);
    expect(state.openFiles[0].path).toBe('/test/file.ts');
    expect(state.activeFile).toBe('/test/file.ts');
  });

  it('does not duplicate when opening same file twice', () => {
    const file = {
      path: '/test/file.ts',
      name: 'file.ts',
      content: 'const x = 1;',
      language: 'typescript',
      modified: false,
    };

    useAppStore.getState().openFile(file);
    useAppStore.getState().openFile(file);

    const state = useAppStore.getState();
    expect(state.openFiles).toHaveLength(1);
    expect(state.activeFile).toBe('/test/file.ts');
  });

  it('closes a file and selects the next tab', () => {
    const file1 = { path: '/a.ts', name: 'a.ts', content: '', language: 'typescript', modified: false };
    const file2 = { path: '/b.ts', name: 'b.ts', content: '', language: 'typescript', modified: false };

    useAppStore.getState().openFile(file1);
    useAppStore.getState().openFile(file2);
    useAppStore.getState().closeFile('/b.ts');

    const state = useAppStore.getState();
    expect(state.openFiles).toHaveLength(1);
    expect(state.activeFile).toBe('/a.ts');
  });

  it('closes last file and sets activeFile to null', () => {
    const file = { path: '/a.ts', name: 'a.ts', content: '', language: 'typescript', modified: false };

    useAppStore.getState().openFile(file);
    useAppStore.getState().closeFile('/a.ts');

    const state = useAppStore.getState();
    expect(state.openFiles).toHaveLength(0);
    expect(state.activeFile).toBeNull();
  });

  it('updates file content and marks as modified', () => {
    const file = { path: '/a.ts', name: 'a.ts', content: 'old', language: 'typescript', modified: false };

    useAppStore.getState().openFile(file);
    useAppStore.getState().updateFileContent('/a.ts', 'new content');

    const state = useAppStore.getState();
    expect(state.openFiles[0].content).toBe('new content');
    expect(state.openFiles[0].modified).toBe(true);
  });

  it('marks file as saved and clears modified flag', () => {
    const file = { path: '/a.ts', name: 'a.ts', content: 'content', language: 'typescript', modified: false };

    useAppStore.getState().openFile(file);
    useAppStore.getState().updateFileContent('/a.ts', 'changed');
    useAppStore.getState().markFileSaved('/a.ts');

    const state = useAppStore.getState();
    expect(state.openFiles[0].modified).toBe(false);
  });
});

describe('Theme', () => {
  it('saves theme to localStorage', () => {
    useAppStore.getState().setTheme('monokai');

    const state = useAppStore.getState();
    expect(state.theme).toBe('monokai');
    expect(window.localStorage.setItem).toHaveBeenCalledWith('localcode-theme', 'monokai');
  });
});

describe('Terminal', () => {
  it('toggles terminal visibility', () => {
    expect(useAppStore.getState().terminalVisible).toBe(true);
    useAppStore.getState().toggleTerminal();
    expect(useAppStore.getState().terminalVisible).toBe(false);
    useAppStore.getState().toggleTerminal();
    expect(useAppStore.getState().terminalVisible).toBe(true);
  });
});

describe('Chat Panel', () => {
  it('toggles chat panel visibility', () => {
    expect(useAppStore.getState().chatPanelVisible).toBe(true);
    useAppStore.getState().toggleChatPanel();
    expect(useAppStore.getState().chatPanelVisible).toBe(false);
  });
});

describe('Sidebar', () => {
  it('switches sidebar view', () => {
    useAppStore.getState().setSidebarView('search');
    expect(useAppStore.getState().sidebarView).toBe('search');

    useAppStore.getState().setSidebarView('git');
    expect(useAppStore.getState().sidebarView).toBe('git');

    useAppStore.getState().setSidebarView('composer');
    expect(useAppStore.getState().sidebarView).toBe('composer');
  });
});

describe('Download Progress', () => {
  it('tracks download progress per model', () => {
    useAppStore.getState().setDownloadProgress('model-1', {
      downloaded: 500,
      total: 1000,
      speed: 100,
      eta: 5,
    });

    const state = useAppStore.getState();
    expect(state.downloadProgress['model-1']).toBeDefined();
    expect(state.downloadProgress['model-1'].downloaded).toBe(500);
  });

  it('removes download progress when set to null', () => {
    useAppStore.getState().setDownloadProgress('model-1', {
      downloaded: 1000,
      total: 1000,
      speed: 100,
      eta: 0,
    });
    useAppStore.getState().setDownloadProgress('model-1', null);

    const state = useAppStore.getState();
    expect(state.downloadProgress['model-1']).toBeUndefined();
  });
});

describe('Chat Messages', () => {
  it('adds chat messages', () => {
    useAppStore.getState().addChatMessage({
      id: 'msg-1',
      role: 'user',
      content: 'Hello',
      timestamp: Date.now(),
    });

    expect(useAppStore.getState().chatMessages).toHaveLength(1);
    expect(useAppStore.getState().chatMessages[0].content).toBe('Hello');
  });

  it('updates chat message', () => {
    useAppStore.getState().addChatMessage({
      id: 'msg-1',
      role: 'assistant',
      content: '',
      timestamp: Date.now(),
    });

    useAppStore.getState().updateChatMessage('msg-1', { content: 'Updated' });

    expect(useAppStore.getState().chatMessages[0].content).toBe('Updated');
  });

  it('clears all chat messages', () => {
    useAppStore.getState().addChatMessage({
      id: 'msg-1',
      role: 'user',
      content: 'test',
      timestamp: Date.now(),
    });
    useAppStore.getState().clearChat();

    expect(useAppStore.getState().chatMessages).toHaveLength(0);
  });
});

describe('Quick Open', () => {
  it('toggles quick open visibility', () => {
    expect(useAppStore.getState().quickOpenVisible).toBe(false);
    useAppStore.getState().toggleQuickOpen();
    expect(useAppStore.getState().quickOpenVisible).toBe(true);
    useAppStore.getState().toggleQuickOpen();
    expect(useAppStore.getState().quickOpenVisible).toBe(false);
  });
});

describe('Completion Status', () => {
  it('tracks completion status', () => {
    expect(useAppStore.getState().completionStatus).toBe('idle');
    useAppStore.getState().setCompletionStatus('completing');
    expect(useAppStore.getState().completionStatus).toBe('completing');
    useAppStore.getState().setCompletionStatus('idle');
    expect(useAppStore.getState().completionStatus).toBe('idle');
  });
});

describe('Editor Selection', () => {
  it('tracks editor selection text', () => {
    useAppStore.getState().setEditorSelection('const x = 1;');
    expect(useAppStore.getState().editorSelection).toBe('const x = 1;');
  });
});

describe('Agent Mode', () => {
  it('toggles agent mode', () => {
    expect(useAppStore.getState().agentMode).toBe(false);
    useAppStore.getState().toggleAgentMode();
    expect(useAppStore.getState().agentMode).toBe(true);
  });
});
