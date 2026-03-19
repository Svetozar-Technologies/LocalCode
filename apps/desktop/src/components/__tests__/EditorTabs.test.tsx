import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import EditorTabs from '../Editor/EditorTabs';
import { useAppStore } from '../../stores/appStore';

beforeEach(() => {
  useAppStore.setState({
    openFiles: [],
    activeFile: null,
  });
});

describe('EditorTabs', () => {
  it('renders nothing when no files are open', () => {
    const { container } = render(<EditorTabs />);
    expect(container.innerHTML).toBe('');
  });

  it('renders tabs for open files', () => {
    useAppStore.setState({
      openFiles: [
        { path: '/a.ts', name: 'a.ts', content: '', language: 'typescript', modified: false },
        { path: '/b.ts', name: 'b.ts', content: '', language: 'typescript', modified: false },
      ],
      activeFile: '/a.ts',
    });

    render(<EditorTabs />);
    expect(screen.getByText('a.ts')).toBeDefined();
    expect(screen.getByText('b.ts')).toBeDefined();
  });

  it('highlights active tab', () => {
    useAppStore.setState({
      openFiles: [
        { path: '/a.ts', name: 'a.ts', content: '', language: 'typescript', modified: false },
      ],
      activeFile: '/a.ts',
    });

    render(<EditorTabs />);
    const tab = document.querySelector('.editor-tab.active');
    expect(tab).toBeDefined();
  });

  it('switches active file on tab click', () => {
    useAppStore.setState({
      openFiles: [
        { path: '/a.ts', name: 'a.ts', content: '', language: 'typescript', modified: false },
        { path: '/b.ts', name: 'b.ts', content: '', language: 'typescript', modified: false },
      ],
      activeFile: '/a.ts',
    });

    render(<EditorTabs />);
    fireEvent.click(screen.getByText('b.ts'));
    expect(useAppStore.getState().activeFile).toBe('/b.ts');
  });
});
