import { create } from 'zustand';
import type { FileEntry, OpenFile, ChatMessage, SidebarView, GitFileStatus, LLMConfig, BottomPanelTab, DiagnosticProblem, OutputEntry, ChatSessionInfo } from '../types';

interface AppState {
  // Project
  projectPath: string | null;
  setProjectPath: (path: string | null) => void;

  // File tree
  fileTree: FileEntry[];
  setFileTree: (tree: FileEntry[]) => void;
  toggleDir: (path: string) => void;

  // Open files / tabs
  openFiles: OpenFile[];
  activeFile: string | null;
  openFile: (file: OpenFile) => void;
  closeFile: (path: string) => void;
  setActiveFile: (path: string) => void;
  updateFileContent: (path: string, content: string) => void;
  markFileSaved: (path: string) => void;

  // Sidebar
  sidebarView: SidebarView;
  setSidebarView: (view: SidebarView) => void;
  sidebarWidth: number;
  setSidebarWidth: (width: number) => void;

  // Terminal / Bottom Panel
  terminalVisible: boolean;
  toggleTerminal: () => void;
  terminalHeight: number;
  setTerminalHeight: (height: number) => void;
  bottomPanelTab: BottomPanelTab;
  setBottomPanelTab: (tab: BottomPanelTab) => void;
  problems: DiagnosticProblem[];
  addProblem: (problem: DiagnosticProblem) => void;
  setProblems: (problems: DiagnosticProblem[]) => void;
  clearProblems: () => void;
  outputLog: OutputEntry[];
  addOutputEntry: (entry: OutputEntry) => void;
  clearOutput: () => void;

  // AI Chat
  chatMessages: ChatMessage[];
  addChatMessage: (msg: ChatMessage) => void;
  updateChatMessage: (id: string, updates: Partial<ChatMessage>) => void;
  clearChat: () => void;
  isAIStreaming: boolean;
  setAIStreaming: (streaming: boolean) => void;

  // Chat Persistence
  activeChatSessionId: string | null;
  chatSessions: ChatSessionInfo[];
  setActiveChatSessionId: (id: string | null) => void;
  setChatSessions: (sessions: ChatSessionInfo[]) => void;

  // LLM
  llmConfig: LLMConfig;
  setLLMConfig: (config: Partial<LLMConfig>) => void;
  llmConnected: boolean;
  setLLMConnected: (connected: boolean) => void;

  // Git
  gitStatus: GitFileStatus[];
  setGitStatus: (status: GitFileStatus[]) => void;
  currentBranch: string;
  setCurrentBranch: (branch: string) => void;

  // Model downloads
  downloadProgress: Record<string, { downloaded: number; total: number; speed: number; eta: number }>;
  setDownloadProgress: (catalogId: string, progress: { downloaded: number; total: number; speed: number; eta: number } | null) => void;

  // Agent mode
  agentMode: boolean;
  toggleAgentMode: () => void;

  // AI Chat panel (right side)
  chatPanelVisible: boolean;
  toggleChatPanel: () => void;
  chatPanelWidth: number;
  setChatPanelWidth: (width: number) => void;

  // Theme
  theme: string;
  setTheme: (theme: string) => void;

  // Inline completion status
  completionStatus: 'idle' | 'completing';
  setCompletionStatus: (status: 'idle' | 'completing') => void;

  // Editor selection (for @selection mention)
  editorSelection: string;
  setEditorSelection: (text: string) => void;

  // Last terminal output (for @terminal mention)
  lastTerminalOutput: string;
  setLastTerminalOutput: (output: string) => void;

  // Quick Open
  quickOpenVisible: boolean;
  setQuickOpenVisible: (visible: boolean) => void;
  toggleQuickOpen: () => void;

  // Auto Save
  autoSave: boolean;
  setAutoSave: (enabled: boolean) => void;
  autoSaveDelay: number;
  setAutoSaveDelay: (delay: number) => void;

  // Inline Edit
  inlineEditVisible: boolean;
  setInlineEditVisible: (visible: boolean) => void;

  // Find & Replace
  showFindReplace: boolean;
  toggleFindReplace: () => void;
  setShowFindReplace: (visible: boolean) => void;

  // Blame View
  showBlameView: boolean;
  blameFilePath: string | null;
  setShowBlameView: (visible: boolean) => void;
  setBlameFilePath: (path: string | null) => void;

  // Split Editor
  splitEditorMode: 'off' | 'horizontal' | 'vertical';
  splitEditorRightPath: string | null;
  setSplitEditorMode: (mode: 'off' | 'horizontal' | 'vertical') => void;
  setSplitEditorRightPath: (path: string | null) => void;

  // Command Palette
  commandPaletteVisible: boolean;
  toggleCommandPalette: () => void;

  // Multi-Model Selector
  selectedProvider: string;
  setSelectedProvider: (provider: string) => void;

  // Pending Agent Changes (Inline Diff Review)
  pendingAgentChanges: Record<string, { original: string; modified: string }>;
  addPendingAgentChange: (path: string, original: string, modified: string) => void;
  removePendingAgentChange: (path: string) => void;
  clearPendingAgentChanges: () => void;

  // Markdown Preview (Feature 4)
  markdownPreviewVisible: boolean;
  toggleMarkdownPreview: () => void;

  // Agent Plan Mode (Feature 7)
  agentPlanMode: boolean;
  setAgentPlanMode: (enabled: boolean) => void;
  agentPlan: string | null;
  setAgentPlan: (plan: string | null) => void;

  // Checkpoints / Session Restore (Feature 11)
  checkpoints: Array<{ id: string; timestamp: number; files: Record<string, string> }>;
  createCheckpoint: () => void;
  restoreCheckpoint: (id: string) => void;

  // App Preview (Feature 13)
  appPreviewVisible: boolean;
  appPreviewUrl: string;
  toggleAppPreview: () => void;
  setAppPreviewUrl: (url: string) => void;

  // Code Review (Feature 15)
  reviewComments: Array<{ file: string; line: number; severity: string; message: string }>;
  setReviewComments: (comments: Array<{ file: string; line: number; severity: string; message: string }>) => void;

  // Codebase Map (Feature 19)
  codebaseMapVisible: boolean;
  toggleCodebaseMap: () => void;

  // Setup Wizard
  setupComplete: boolean;
  showSetupWizard: boolean;
  setSetupComplete: (complete: boolean) => void;
  setShowSetupWizard: (visible: boolean) => void;
  isLLMConfigured: () => boolean;

  // Toast Notifications
  toasts: Array<{ id: string; message: string; type: 'info' | 'success' | 'error' }>;
  addToast: (message: string, type?: 'info' | 'success' | 'error') => void;
  removeToast: (id: string) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  // Project
  projectPath: null,
  setProjectPath: (path) => set({ projectPath: path }),

  // File tree
  fileTree: [],
  setFileTree: (tree) => set({ fileTree: tree }),
  toggleDir: (path) =>
    set((state) => ({
      fileTree: toggleDirRecursive(state.fileTree, path),
    })),

  // Open files
  openFiles: [],
  activeFile: null,
  openFile: (file) =>
    set((state) => {
      const exists = state.openFiles.find((f) => f.path === file.path);
      if (exists) return { activeFile: file.path };
      return {
        openFiles: [...state.openFiles, file],
        activeFile: file.path,
      };
    }),
  closeFile: (path) =>
    set((state) => {
      const files = state.openFiles.filter((f) => f.path !== path);
      const active =
        state.activeFile === path
          ? files.length > 0
            ? files[files.length - 1].path
            : null
          : state.activeFile;
      return { openFiles: files, activeFile: active };
    }),
  setActiveFile: (path) => set({ activeFile: path }),
  updateFileContent: (path, content) =>
    set((state) => ({
      openFiles: state.openFiles.map((f) =>
        f.path === path ? { ...f, content, modified: true } : f
      ),
    })),
  markFileSaved: (path) =>
    set((state) => ({
      openFiles: state.openFiles.map((f) =>
        f.path === path ? { ...f, modified: false } : f
      ),
    })),

  // Sidebar
  sidebarView: 'explorer',
  setSidebarView: (view) => set({ sidebarView: view }),
  sidebarWidth: 260,
  setSidebarWidth: (width) => set({ sidebarWidth: width }),

  // Terminal / Bottom Panel
  terminalVisible: true,
  toggleTerminal: () => set((state) => ({ terminalVisible: !state.terminalVisible })),
  terminalHeight: 200,
  setTerminalHeight: (height) => set({ terminalHeight: height }),
  bottomPanelTab: 'terminal',
  setBottomPanelTab: (tab) => set({ bottomPanelTab: tab }),
  problems: [],
  addProblem: (problem) =>
    set((state) => ({ problems: [...state.problems, problem] })),
  setProblems: (problems) => set({ problems }),
  clearProblems: () => set({ problems: [] }),
  outputLog: [],
  addOutputEntry: (entry) =>
    set((state) => ({ outputLog: [...state.outputLog, entry] })),
  clearOutput: () => set({ outputLog: [] }),

  // AI Chat
  chatMessages: [],
  addChatMessage: (msg) =>
    set((state) => ({ chatMessages: [...state.chatMessages, msg] })),
  updateChatMessage: (id, updates) =>
    set((state) => ({
      chatMessages: state.chatMessages.map((m) =>
        m.id === id ? { ...m, ...updates } : m
      ),
    })),
  clearChat: () => set({ chatMessages: [] }),
  isAIStreaming: false,
  setAIStreaming: (streaming) => set({ isAIStreaming: streaming }),

  // Chat Persistence
  activeChatSessionId: null,
  chatSessions: [],
  setActiveChatSessionId: (id) => set({ activeChatSessionId: id }),
  setChatSessions: (sessions) => set({ chatSessions: sessions }),

  // LLM
  llmConfig: {
    modelPath: '',
    modelName: 'No model loaded',
    contextSize: 4096,
    gpuLayers: 99,
    temperature: 0.7,
  },
  setLLMConfig: (config) =>
    set((state) => ({ llmConfig: { ...state.llmConfig, ...config } })),
  llmConnected: false,
  setLLMConnected: (connected) => set({ llmConnected: connected }),

  // Git
  gitStatus: [],
  setGitStatus: (status) => set({ gitStatus: status }),
  currentBranch: '',
  setCurrentBranch: (branch) => set({ currentBranch: branch }),

  // Model downloads
  downloadProgress: {},
  setDownloadProgress: (catalogId, progress) =>
    set((state) => {
      const next = { ...state.downloadProgress };
      if (progress === null) {
        delete next[catalogId];
      } else {
        next[catalogId] = progress;
      }
      return { downloadProgress: next };
    }),

  // Agent
  agentMode: false,
  toggleAgentMode: () => set((state) => ({ agentMode: !state.agentMode })),

  // AI Chat panel (right side)
  chatPanelVisible: true,
  toggleChatPanel: () => set((state) => ({ chatPanelVisible: !state.chatPanelVisible })),
  chatPanelWidth: 320,
  setChatPanelWidth: (width) => set({ chatPanelWidth: width }),

  // Theme
  theme: localStorage.getItem('localcode-theme') || 'dark',
  setTheme: (theme) => {
    localStorage.setItem('localcode-theme', theme);
    set({ theme });
  },

  // Inline completion status
  completionStatus: 'idle',
  setCompletionStatus: (status) => set({ completionStatus: status }),

  // Editor selection
  editorSelection: '',
  setEditorSelection: (text) => set({ editorSelection: text }),

  // Last terminal output
  lastTerminalOutput: '',
  setLastTerminalOutput: (output) => set({ lastTerminalOutput: output }),

  // Quick Open
  quickOpenVisible: false,
  setQuickOpenVisible: (visible) => set({ quickOpenVisible: visible }),
  toggleQuickOpen: () => set((state) => ({ quickOpenVisible: !state.quickOpenVisible })),

  // Auto Save
  autoSave: localStorage.getItem('localcode-autosave') !== 'false',
  setAutoSave: (enabled) => {
    localStorage.setItem('localcode-autosave', String(enabled));
    set({ autoSave: enabled });
  },
  autoSaveDelay: Number(localStorage.getItem('localcode-autosave-delay')) || 1000,
  setAutoSaveDelay: (delay) => {
    localStorage.setItem('localcode-autosave-delay', String(delay));
    set({ autoSaveDelay: delay });
  },

  // Inline Edit
  inlineEditVisible: false,
  setInlineEditVisible: (visible) => set({ inlineEditVisible: visible }),

  // Find & Replace
  showFindReplace: false,
  toggleFindReplace: () => set((state) => ({ showFindReplace: !state.showFindReplace })),
  setShowFindReplace: (visible) => set({ showFindReplace: visible }),

  // Blame View
  showBlameView: false,
  blameFilePath: null,
  setShowBlameView: (visible) => set({ showBlameView: visible }),
  setBlameFilePath: (path) => set({ blameFilePath: path }),

  // Split Editor
  splitEditorMode: 'off',
  splitEditorRightPath: null,
  setSplitEditorMode: (mode) => set({ splitEditorMode: mode }),
  setSplitEditorRightPath: (path) => set({ splitEditorRightPath: path }),

  // Command Palette
  commandPaletteVisible: false,
  toggleCommandPalette: () => set((state) => ({ commandPaletteVisible: !state.commandPaletteVisible })),

  // Multi-Model Selector
  selectedProvider: localStorage.getItem('localcode-selected-provider') || 'local',
  setSelectedProvider: (provider) => {
    localStorage.setItem('localcode-selected-provider', provider);
    set({ selectedProvider: provider });
  },

  // Pending Agent Changes (Inline Diff Review)
  pendingAgentChanges: {},
  addPendingAgentChange: (path, original, modified) =>
    set((state) => ({
      pendingAgentChanges: { ...state.pendingAgentChanges, [path]: { original, modified } },
    })),
  removePendingAgentChange: (path) =>
    set((state) => {
      const next = { ...state.pendingAgentChanges };
      delete next[path];
      return { pendingAgentChanges: next };
    }),
  clearPendingAgentChanges: () => set({ pendingAgentChanges: {} }),

  // Markdown Preview
  markdownPreviewVisible: false,
  toggleMarkdownPreview: () => set((state) => ({ markdownPreviewVisible: !state.markdownPreviewVisible })),

  // Agent Plan Mode
  agentPlanMode: false,
  setAgentPlanMode: (enabled) => set({ agentPlanMode: enabled }),
  agentPlan: null,
  setAgentPlan: (plan) => set({ agentPlan: plan }),

  // Checkpoints
  checkpoints: [],
  createCheckpoint: () =>
    set((state) => {
      const files: Record<string, string> = {};
      state.openFiles.forEach((f) => { files[f.path] = f.content; });
      const cp = {
        id: `cp-${Date.now()}`,
        timestamp: Date.now(),
        files,
      };
      return { checkpoints: [cp, ...state.checkpoints].slice(0, 5) };
    }),
  restoreCheckpoint: (id) =>
    set((state) => {
      const cp = state.checkpoints.find((c) => c.id === id);
      if (!cp) return {};
      return {
        openFiles: state.openFiles.map((f) =>
          cp.files[f.path] !== undefined
            ? { ...f, content: cp.files[f.path], modified: true }
            : f
        ),
      };
    }),

  // App Preview
  appPreviewVisible: false,
  appPreviewUrl: 'http://localhost:3000',
  toggleAppPreview: () => set((state) => ({ appPreviewVisible: !state.appPreviewVisible })),
  setAppPreviewUrl: (url) => set({ appPreviewUrl: url }),

  // Code Review
  reviewComments: [],
  setReviewComments: (comments) => set({ reviewComments: comments }),

  // Codebase Map
  codebaseMapVisible: false,
  toggleCodebaseMap: () => set((state) => ({ codebaseMapVisible: !state.codebaseMapVisible })),

  // Setup Wizard
  setupComplete: localStorage.getItem('localcode-setup-complete') === 'true',
  showSetupWizard: false,
  setSetupComplete: (complete) => {
    localStorage.setItem('localcode-setup-complete', String(complete));
    set({ setupComplete: complete });
  },
  setShowSetupWizard: (visible) => set({ showSetupWizard: visible }),
  isLLMConfigured: (): boolean => {
    const state = get();
    if (state.selectedProvider === 'local') {
      return state.llmConnected || state.llmConfig.modelPath !== '';
    }
    // Cloud providers: check if API key is stored in localStorage
    const providerConfigs = localStorage.getItem('localcode-llm-providers');
    if (providerConfigs) {
      try {
        const providers = JSON.parse(providerConfigs);
        return providers.some((p: { enabled: boolean; apiKey: string }) => p.enabled && p.apiKey);
      } catch { /* ignore */ }
    }
    return state.llmConnected;
  },

  // Toast Notifications
  toasts: [],
  addToast: (message, type = 'info') => {
    const id = `toast-${Date.now()}`;
    set((state) => ({
      toasts: [...state.toasts, { id, message, type }],
    }));
    setTimeout(() => {
      useAppStore.getState().removeToast(id);
    }, 4000);
  },
  removeToast: (id) =>
    set((state) => ({
      toasts: state.toasts.filter((t) => t.id !== id),
    })),
}));

function toggleDirRecursive(tree: FileEntry[], path: string): FileEntry[] {
  return tree.map((entry) => {
    if (entry.path === path) {
      return { ...entry, expanded: !entry.expanded };
    }
    if (entry.children) {
      return { ...entry, children: toggleDirRecursive(entry.children, path) };
    }
    return entry;
  });
}
