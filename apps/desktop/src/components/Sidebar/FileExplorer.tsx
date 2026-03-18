import { useCallback } from 'react';
import { useAppStore } from '../../stores/appStore';
import type { FileEntry } from '../../types';
import { invoke } from '@tauri-apps/api/core';

const EXT_COLORS: Record<string, string> = {
  ts: '#3178c6', tsx: '#3178c6', js: '#f7df1e', jsx: '#f7df1e',
  py: '#3776ab', rs: '#dea584', go: '#00add8', java: '#b07219',
  html: '#e34c26', css: '#1572b6', scss: '#c6538c', json: '#292929',
  md: '#083fa1', yml: '#cb171e', yaml: '#cb171e', toml: '#9c4221',
  sh: '#89e051', bash: '#89e051', sql: '#e38c00', graphql: '#e10098',
  c: '#555555', cpp: '#f34b7d', h: '#555555', swift: '#f05138',
  kt: '#A97BFF', rb: '#701516', php: '#4F5D95', lua: '#000080',
  zig: '#ec915c', svelte: '#ff3e00', vue: '#41b883',
};

function getFileColor(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase() || '';
  return EXT_COLORS[ext] || 'var(--text-secondary)';
}

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact',
    py: 'python', rs: 'rust', go: 'go', java: 'java', c: 'c', cpp: 'cpp', h: 'c',
    html: 'html', css: 'css', scss: 'scss', json: 'json', md: 'markdown',
    yml: 'yaml', yaml: 'yaml', toml: 'toml', sh: 'shell', sql: 'sql',
    xml: 'xml', svg: 'xml', graphql: 'graphql', swift: 'swift', kt: 'kotlin',
    rb: 'ruby', php: 'php', lua: 'lua', zig: 'zig', svelte: 'svelte', vue: 'vue',
  };
  return map[ext] || 'plaintext';
}

interface FileTreeItemProps {
  entry: FileEntry;
  depth: number;
}

function FileTreeItem({ entry, depth }: FileTreeItemProps) {
  const { toggleDir, openFile, activeFile, gitStatus } = useAppStore();

  const handleClick = useCallback(async () => {
    if (entry.is_dir) {
      if (!entry.children || entry.children.length === 0) {
        try {
          const children = await invoke<FileEntry[]>('read_dir', { path: entry.path });
          const store = useAppStore.getState();
          const updateTree = (tree: FileEntry[]): FileEntry[] =>
            tree.map((e) => {
              if (e.path === entry.path) return { ...e, children, expanded: true };
              if (e.children) return { ...e, children: updateTree(e.children) };
              return e;
            });
          store.setFileTree(updateTree(store.fileTree));
        } catch {
          toggleDir(entry.path);
        }
      } else {
        toggleDir(entry.path);
      }
    } else {
      try {
        const content = await invoke<string>('read_file', { path: entry.path });
        openFile({
          path: entry.path,
          name: entry.name,
          content,
          language: getLanguageFromPath(entry.path),
          modified: false,
        });
      } catch (err) {
        console.error('Failed to read file:', err);
      }
    }
  }, [entry, toggleDir, openFile]);

  const gitEntry = gitStatus.find((g) => entry.path.endsWith(g.path));
  const gitClass = gitEntry ? `git-${gitEntry.status}` : '';

  return (
    <>
      <div
        className={`file-tree-item ${activeFile === entry.path ? 'active' : ''}`}
        style={{ paddingLeft: `${8 + depth * 16}px` }}
        onClick={handleClick}
      >
        {entry.is_dir && (
          <span className={`chevron ${entry.expanded ? 'expanded' : ''}`}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" fill="none" />
            </svg>
          </span>
        )}
        <span className="icon" style={{ color: entry.is_dir ? 'var(--accent-yellow)' : getFileColor(entry.name) }}>
          {entry.is_dir ? (
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M14.5 3H7.71l-.85-.85L6.51 2h-5l-.5.5v11l.5.5h13l.5-.5v-10L14.5 3zm-.51 8.49V13h-12V3h4.29l.85.85.36.15H14v7.49z" />
            </svg>
          ) : (
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M13.71 4.29l-3-3L10 1H4L3 2v12l1 1h9l1-1V5l-.29-.71zM13 14H4V2h5v4h4v8z" />
            </svg>
          )}
        </span>
        <span className="name">{entry.name}</span>
        {gitEntry && <span className={`git-badge ${gitClass}`}>{gitEntry.status[0].toUpperCase()}</span>}
      </div>
      {entry.is_dir && entry.expanded && entry.children?.map((child) => (
        <FileTreeItem key={child.path} entry={child} depth={depth + 1} />
      ))}
    </>
  );
}

export default function FileExplorer() {
  const { fileTree, projectPath } = useAppStore();

  if (!projectPath) {
    return (
      <div style={{ padding: '20px 12px', textAlign: 'center', color: 'var(--text-muted)' }}>
        <p style={{ marginBottom: 12 }}>No folder opened</p>
        <button className="open-folder-btn" onClick={async () => {
          try {
            const { open } = await import('@tauri-apps/plugin-dialog');
            const selected = await open({ directory: true });
            if (selected) {
              const path = selected as string;
              useAppStore.getState().setProjectPath(path);
              const tree = await invoke<FileEntry[]>('read_dir', { path });
              useAppStore.getState().setFileTree(tree);
            }
          } catch (err) {
            console.error('Failed to open folder:', err);
          }
        }}>
          Open Folder
        </button>
      </div>
    );
  }

  return (
    <div className="sidebar-content">
      {fileTree.map((entry) => (
        <FileTreeItem key={entry.path} entry={entry} depth={0} />
      ))}
    </div>
  );
}
