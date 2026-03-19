import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import StatusBar from '../StatusBar/StatusBar';
import { useAppStore } from '../../stores/appStore';

beforeEach(() => {
  useAppStore.setState({
    activeFile: null,
    openFiles: [],
    currentBranch: 'main',
    llmConnected: false,
    llmConfig: { modelPath: '', modelName: 'No model loaded', contextSize: 4096, gpuLayers: 99, temperature: 0.7 },
    agentMode: false,
    problems: [],
    completionStatus: 'idle',
  });
});

describe('StatusBar', () => {
  it('renders without crashing', () => {
    render(<StatusBar />);
    expect(screen.getByText('LocalCode v0.2.0')).toBeDefined();
  });

  it('shows branch name', () => {
    useAppStore.setState({ currentBranch: 'feature/test' });
    render(<StatusBar />);
    expect(screen.getByText('feature/test')).toBeDefined();
  });

  it('shows default branch when empty', () => {
    useAppStore.setState({ currentBranch: '' });
    render(<StatusBar />);
    expect(screen.getByText('main')).toBeDefined();
  });

  it('shows Copilot indicator when LLM connected', () => {
    useAppStore.setState({ llmConnected: true });
    render(<StatusBar />);
    expect(screen.getByText('Copilot')).toBeDefined();
  });

  it('shows Agent Mode badge when active', () => {
    useAppStore.setState({ agentMode: true });
    render(<StatusBar />);
    expect(screen.getByText('Agent Mode')).toBeDefined();
  });

  it('shows file info when a file is open', () => {
    useAppStore.setState({
      activeFile: '/test/file.ts',
      openFiles: [{
        path: '/test/file.ts',
        name: 'file.ts',
        content: 'line1\nline2\nline3',
        language: 'typescript',
        modified: false,
      }],
    });
    render(<StatusBar />);
    expect(screen.getByText('TS')).toBeDefined();
    expect(screen.getByText('UTF-8')).toBeDefined();
    expect(screen.getByText('3 lines')).toBeDefined();
  });
});
