export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileEntry[];
  expanded?: boolean;
}

export interface OpenFile {
  path: string;
  name: string;
  content: string;
  language: string;
  modified: boolean;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  agentSteps?: AgentStep[];
}

export interface AgentStep {
  type: 'thinking' | 'tool_call' | 'tool_result' | 'response';
  tool?: string;
  args?: Record<string, unknown>;
  result?: string;
  content?: string;
  timestamp: number;
}

export interface LLMConfig {
  modelPath: string;
  modelName: string;
  contextSize: number;
  gpuLayers: number;
  temperature: number;
}

export interface GitFileStatus {
  path: string;
  status: 'modified' | 'added' | 'deleted' | 'untracked' | 'renamed';
}

export interface SearchResult {
  file: string;
  line: number;
  column: number;
  content: string;
  matchLength: number;
}

export type SidebarView = 'explorer' | 'search' | 'git' | 'ai';

export interface TerminalSession {
  id: string;
  title: string;
}
