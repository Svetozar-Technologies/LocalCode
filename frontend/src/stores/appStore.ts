import { create } from 'zustand';
import type { FileEntry, OpenFile, ChatMessage, SidebarView, GitFileStatus, LLMConfig } from '../types';

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

  // Terminal
  terminalVisible: boolean;
  toggleTerminal: () => void;
  terminalHeight: number;
  setTerminalHeight: (height: number) => void;

  // AI Chat
  chatMessages: ChatMessage[];
  addChatMessage: (msg: ChatMessage) => void;
  updateChatMessage: (id: string, updates: Partial<ChatMessage>) => void;
  clearChat: () => void;
  isAIStreaming: boolean;
  setAIStreaming: (streaming: boolean) => void;

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

  // Agent mode
  agentMode: boolean;
  toggleAgentMode: () => void;
}

export const useAppStore = create<AppState>((set) => ({
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

  // Terminal
  terminalVisible: true,
  toggleTerminal: () => set((state) => ({ terminalVisible: !state.terminalVisible })),
  terminalHeight: 200,
  setTerminalHeight: (height) => set({ terminalHeight: height }),

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

  // Agent
  agentMode: false,
  toggleAgentMode: () => set((state) => ({ agentMode: !state.agentMode })),
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
