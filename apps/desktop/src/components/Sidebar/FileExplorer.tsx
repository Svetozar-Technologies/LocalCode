import { useCallback, useEffect, useState, useRef } from 'react';
import { useAppStore } from '../../stores/appStore';
import type { FileEntry, GitFileStatus, OpenFile } from '../../types';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

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

const IMAGE_EXTS = new Set(['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico', 'bmp']);

// File templates for new files based on extension
function getFileTemplate(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() || '';
  switch (ext) {
    case 'py':
      return '#!/usr/bin/env python3\n\n\ndef main():\n    pass\n\n\nif __name__ == "__main__":\n    main()\n';
    case 'rs':
      return 'fn main() {\n    println!("Hello, world!");\n}\n';
    case 'go':
      return 'package main\n\nimport "fmt"\n\nfunc main() {\n\tfmt.Println("Hello, world!")\n}\n';
    case 'js':
      return '\'use strict\';\n\n';
    case 'ts':
      return '\n';
    case 'tsx':
      return 'export default function Component() {\n  return (\n    <div>\n\n    </div>\n  );\n}\n';
    case 'jsx':
      return 'export default function Component() {\n  return (\n    <div>\n\n    </div>\n  );\n}\n';
    case 'java': {
      const className = filename.replace('.java', '');
      return `public class ${className} {\n    public static void main(String[] args) {\n\n    }\n}\n`;
    }
    case 'c':
      return '#include <stdio.h>\n\nint main() {\n    printf("Hello, world!\\n");\n    return 0;\n}\n';
    case 'cpp':
      return '#include <iostream>\n\nint main() {\n    std::cout << "Hello, world!" << std::endl;\n    return 0;\n}\n';
    case 'html':
      return '<!DOCTYPE html>\n<html lang="en">\n<head>\n  <meta charset="UTF-8">\n  <meta name="viewport" content="width=device-width, initial-scale=1.0">\n  <title>Document</title>\n</head>\n<body>\n\n</body>\n</html>\n';
    case 'css':
      return '/* Styles */\n\n';
    case 'sh':
    case 'bash':
      return '#!/bin/bash\n\n';
    case 'rb':
      return '# frozen_string_literal: true\n\n';
    case 'swift':
      return 'import Foundation\n\n';
    case 'kt':
      return 'fun main() {\n    println("Hello, world!")\n}\n';
    case 'lua':
      return '-- Main\n\n';
    case 'php':
      return '<?php\n\n';
    default:
      return '';
  }
}

function isImageFile(name: string): boolean {
  const ext = name.split('.').pop()?.toLowerCase() || '';
  return IMAGE_EXTS.has(ext);
}

// Context menu state
interface ContextMenuState {
  x: number;
  y: number;
  entry: FileEntry | null;
  isRoot: boolean;
}

// Inline input state (for new file/folder or rename)
interface InlineInputState {
  parentPath: string;
  type: 'file' | 'folder' | 'rename';
  initialValue: string;
  entryPath?: string; // for rename, the original path
}

interface FileTreeItemProps {
  entry: FileEntry;
  depth: number;
  onContextMenu: (e: React.MouseEvent, entry: FileEntry) => void;
  inlineInput: InlineInputState | null;
  onInlineSubmit: (value: string) => void;
  onInlineCancel: () => void;
}

function FileTreeItem({ entry, depth, onContextMenu, inlineInput, onInlineSubmit, onInlineCancel }: FileTreeItemProps) {
  const { toggleDir, openFile, activeFile, gitStatus } = useAppStore();
  const [isRenaming, setIsRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState('');
  const renameRef = useRef<HTMLInputElement>(null);
  const inlineRef = useRef<HTMLInputElement>(null);

  // Handle rename mode
  const shouldRename = inlineInput?.type === 'rename' && inlineInput.entryPath === entry.path;
  useEffect(() => {
    if (shouldRename) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setIsRenaming(true);
      setRenameValue(inlineInput!.initialValue);
    } else {
      setIsRenaming(false);
    }
  }, [shouldRename, inlineInput]);

  useEffect(() => {
    if (isRenaming && renameRef.current) {
      renameRef.current.focus();
      const dotIdx = renameValue.lastIndexOf('.');
      renameRef.current.setSelectionRange(0, dotIdx > 0 ? dotIdx : renameValue.length);
    }
  }, [isRenaming, renameValue]);

  // Focus inline new file/folder input
  useEffect(() => {
    if (inlineRef.current) {
      inlineRef.current.focus();
    }
  });

  const handleClick = useCallback(async () => {
    if (isRenaming) return;
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
        if (isImageFile(entry.name)) {
          openFile({
            path: entry.path,
            name: entry.name,
            content: '',
            language: 'image',
            modified: false,
          });
        } else {
          const content = await invoke<string>('read_file', { path: entry.path });
          openFile({
            path: entry.path,
            name: entry.name,
            content,
            language: getLanguageFromPath(entry.path),
            modified: false,
          });
        }
      } catch (err) {
        console.error('Failed to read file:', err);
      }
    }
  }, [entry, toggleDir, openFile, isRenaming]);

  const handleRenameSubmit = () => {
    const trimmed = renameValue.trim();
    if (trimmed && trimmed !== entry.name) {
      onInlineSubmit(trimmed);
    } else {
      onInlineCancel();
    }
  };

  const gitEntry = gitStatus.find((g: GitFileStatus) => entry.path.endsWith(g.path));
  const gitClass = gitEntry ? `git-${gitEntry.status}` : '';

  // Show inline input for new file/folder INSIDE this directory
  const showInlineInput = inlineInput &&
    (inlineInput.type === 'file' || inlineInput.type === 'folder') &&
    inlineInput.parentPath === entry.path &&
    entry.is_dir;

  return (
    <>
      <div
        className={`file-tree-item ${activeFile === entry.path ? 'active' : ''}`}
        style={{ paddingLeft: `${8 + depth * 16}px` }}
        onClick={handleClick}
        onContextMenu={(e) => onContextMenu(e, entry)}
      >
        {/* Indent guides */}
        {depth > 0 && Array.from({ length: depth }).map((_, i) => (
          <span
            key={i}
            className="indent-guide"
            style={{ left: `${8 + i * 16 + 8}px` }}
          />
        ))}
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
        {isRenaming ? (
          <input
            ref={renameRef}
            className="inline-rename-input"
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onBlur={handleRenameSubmit}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleRenameSubmit();
              if (e.key === 'Escape') onInlineCancel();
            }}
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <span className="name">{entry.name}</span>
        )}
        {!isRenaming && gitEntry && <span className={`git-badge ${gitClass}`}>{gitEntry.status[0].toUpperCase()}</span>}
      </div>

      {/* Inline input for new file/folder inside this expanded directory */}
      {showInlineInput && entry.expanded && (
        <InlineNewInput
          depth={depth + 1}
          type={inlineInput!.type as 'file' | 'folder'}
          onSubmit={onInlineSubmit}
          onCancel={onInlineCancel}
        />
      )}

      {entry.is_dir && entry.expanded && entry.children?.map((child) => (
        <FileTreeItem
          key={child.path}
          entry={child}
          depth={depth + 1}
          onContextMenu={onContextMenu}
          inlineInput={inlineInput}
          onInlineSubmit={onInlineSubmit}
          onInlineCancel={onInlineCancel}
        />
      ))}
    </>
  );
}

// Inline input component for new file/folder
function InlineNewInput({ depth, type, onSubmit, onCancel }: {
  depth: number;
  type: 'file' | 'folder';
  onSubmit: (value: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleSubmit = () => {
    const trimmed = value.trim();
    if (trimmed) {
      onSubmit(trimmed);
    } else {
      onCancel();
    }
  };

  return (
    <div
      className="file-tree-item inline-new"
      style={{ paddingLeft: `${8 + depth * 16}px` }}
    >
      <span className="icon" style={{ color: type === 'folder' ? 'var(--accent-yellow)' : 'var(--text-secondary)' }}>
        {type === 'folder' ? (
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M14.5 3H7.71l-.85-.85L6.51 2h-5l-.5.5v11l.5.5h13l.5-.5v-10L14.5 3zm-.51 8.49V13h-12V3h4.29l.85.85.36.15H14v7.49z" />
          </svg>
        ) : (
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M13.71 4.29l-3-3L10 1H4L3 2v12l1 1h9l1-1V5l-.29-.71zM13 14H4V2h5v4h4v8z" />
          </svg>
        )}
      </span>
      <input
        ref={inputRef}
        className="inline-rename-input"
        value={value}
        placeholder={type === 'folder' ? 'Folder name' : 'File name'}
        onChange={(e) => setValue(e.target.value)}
        onBlur={handleSubmit}
        onKeyDown={(e) => {
          if (e.key === 'Enter') handleSubmit();
          if (e.key === 'Escape') onCancel();
        }}
      />
    </div>
  );
}

// Context Menu component
function ContextMenu({ state, onClose, onAction }: {
  state: ContextMenuState;
  onClose: () => void;
  onAction: (action: string) => void;
}) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  const items: { label: string; action: string; separator?: boolean; danger?: boolean }[] = [];

  if (state.entry?.is_dir || state.isRoot) {
    items.push({ label: 'New File', action: 'newFile' });
    items.push({ label: 'New Folder', action: 'newFolder' });
    items.push({ label: '', action: '', separator: true });
  }

  if (state.entry && !state.isRoot) {
    items.push({ label: 'Rename', action: 'rename' });
    items.push({ label: 'Delete', action: 'delete', danger: true });
    items.push({ label: '', action: '', separator: true });
  }

  items.push({ label: 'Copy Path', action: 'copyPath' });
  items.push({ label: 'Copy Relative Path', action: 'copyRelPath' });

  if (state.entry) {
    items.push({ label: 'Reveal in Finder', action: 'revealInFinder' });
  }

  return (
    <div
      ref={menuRef}
      className="context-menu"
      style={{ top: state.y, left: state.x }}
    >
      {items.map((item, i) =>
        item.separator ? (
          <div key={i} className="context-menu-separator" />
        ) : (
          <div
            key={i}
            className={`context-menu-item ${item.danger ? 'danger' : ''}`}
            onClick={() => {
              onAction(item.action);
              onClose();
            }}
          >
            {item.label}
          </div>
        )
      )}
    </div>
  );
}

// Delete confirmation dialog
function DeleteDialog({ entry, onConfirm, onCancel }: {
  entry: FileEntry;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="delete-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="delete-dialog-title">Delete {entry.is_dir ? 'Folder' : 'File'}</div>
        <div className="delete-dialog-message">
          Are you sure you want to delete <strong>{entry.name}</strong>?
          {entry.is_dir && <span> This will delete all contents.</span>}
        </div>
        <div className="delete-dialog-actions">
          <button className="btn-secondary" onClick={onCancel}>Cancel</button>
          <button className="btn-danger" onClick={onConfirm}>Delete</button>
        </div>
      </div>
    </div>
  );
}

export default function FileExplorer() {
  const { fileTree, projectPath, setFileTree, openFile } = useAppStore();
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [inlineInput, setInlineInput] = useState<InlineInputState | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<FileEntry | null>(null);

  const projectName = projectPath?.split('/').pop() || '';

  // Listen for external file changes and refresh tree
  useEffect(() => {
    if (!projectPath) return;
    const unlisten = listen<{ path: string; kind: string }>('file-changed', async (event) => {
      const kind = event.payload.kind;
      if (kind === 'create' || kind === 'remove' || kind === 'rename') {
        try {
          const tree = await invoke<FileEntry[]>('read_dir', { path: projectPath });
          useAppStore.getState().setFileTree(tree);
        } catch {
          // ignore
        }
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [projectPath]);

  const refreshTree = useCallback(async () => {
    if (!projectPath) return;
    try {
      const tree = await invoke<FileEntry[]>('read_dir', { path: projectPath });
      setFileTree(tree);
    } catch (err) {
      console.error('Failed to refresh tree:', err);
    }
  }, [projectPath, setFileTree]);

  const collapseAll = useCallback(() => {
    const collapse = (entries: FileEntry[]): FileEntry[] =>
      entries.map((e) => ({
        ...e,
        expanded: false,
        children: e.children ? collapse(e.children) : undefined,
      }));
    setFileTree(collapse(fileTree));
  }, [fileTree, setFileTree]);

  // Ensure parent folder is expanded before showing inline input
  const ensureExpanded = useCallback(async (dirPath: string) => {
    const store = useAppStore.getState();
    const findEntry = (entries: FileEntry[]): FileEntry | null => {
      for (const e of entries) {
        if (e.path === dirPath) return e;
        if (e.children) {
          const found = findEntry(e.children);
          if (found) return found;
        }
      }
      return null;
    };
    const dir = findEntry(store.fileTree);
    if (dir && !dir.expanded) {
      try {
        const children = await invoke<FileEntry[]>('read_dir', { path: dirPath });
        const updateTree = (tree: FileEntry[]): FileEntry[] =>
          tree.map((e) => {
            if (e.path === dirPath) return { ...e, children, expanded: true };
            if (e.children) return { ...e, children: updateTree(e.children) };
            return e;
          });
        store.setFileTree(updateTree(store.fileTree));
      } catch {
        // fallback: just toggle
        store.toggleDir(dirPath);
      }
    }
  }, []);

  const handleContextMenu = useCallback((e: React.MouseEvent, entry: FileEntry) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, entry, isRoot: false });
  }, []);

  const handleRootContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, entry: null, isRoot: true });
  }, []);

  const handleContextAction = useCallback(async (action: string) => {
    if (!contextMenu || !projectPath) return;
    const entry = contextMenu.entry;
    const targetPath = entry?.path || projectPath;
    const parentDir = entry?.is_dir ? entry.path : (entry ? entry.path.substring(0, entry.path.lastIndexOf('/')) : projectPath);

    switch (action) {
      case 'newFile': {
        await ensureExpanded(parentDir);
        setInlineInput({ parentPath: parentDir, type: 'file', initialValue: '' });
        break;
      }
      case 'newFolder': {
        await ensureExpanded(parentDir);
        setInlineInput({ parentPath: parentDir, type: 'folder', initialValue: '' });
        break;
      }
      case 'rename': {
        if (entry) {
          setInlineInput({
            parentPath: entry.path.substring(0, entry.path.lastIndexOf('/')),
            type: 'rename',
            initialValue: entry.name,
            entryPath: entry.path,
          });
        }
        break;
      }
      case 'delete': {
        if (entry) setDeleteTarget(entry);
        break;
      }
      case 'copyPath': {
        navigator.clipboard.writeText(targetPath);
        break;
      }
      case 'copyRelPath': {
        const rel = targetPath.replace(projectPath + '/', '');
        navigator.clipboard.writeText(rel);
        break;
      }
      case 'revealInFinder': {
        try {
          const { open } = await import('@tauri-apps/plugin-shell');
          const dir = entry?.is_dir ? targetPath : targetPath.substring(0, targetPath.lastIndexOf('/'));
          await open(dir);
        } catch {
          // fallback: just ignore
        }
        break;
      }
    }
  }, [contextMenu, projectPath, ensureExpanded]);

  const handleInlineSubmit = useCallback(async (value: string) => {
    if (!inlineInput || !projectPath) return;
    try {
      if (inlineInput.type === 'file') {
        const fullPath = `${inlineInput.parentPath}/${value}`;
        await invoke('create_file', { path: fullPath });
        // Write template content if available
        const template = getFileTemplate(value);
        if (template) {
          await invoke('write_file', { path: fullPath, content: template });
        }
        await refreshTree();
        // Open the new file
        const content = await invoke<string>('read_file', { path: fullPath });
        openFile({
          path: fullPath,
          name: value,
          content,
          language: getLanguageFromPath(fullPath),
          modified: false,
        });
      } else if (inlineInput.type === 'folder') {
        const fullPath = `${inlineInput.parentPath}/${value}`;
        await invoke('create_dir', { path: fullPath });
        await refreshTree();
      } else if (inlineInput.type === 'rename' && inlineInput.entryPath) {
        const parentDir = inlineInput.entryPath.substring(0, inlineInput.entryPath.lastIndexOf('/'));
        const newPath = `${parentDir}/${value}`;
        await invoke('rename_entry', { oldPath: inlineInput.entryPath, newPath });
        await refreshTree();
      }
    } catch (err) {
      console.error('File operation failed:', err);
    }
    setInlineInput(null);
  }, [inlineInput, projectPath, refreshTree, openFile]);

  const handleInlineCancel = useCallback(() => {
    setInlineInput(null);
  }, []);

  const handleDelete = useCallback(async () => {
    if (!deleteTarget) return;
    try {
      await invoke('delete_entry', { path: deleteTarget.path });
      // Close file if it was open
      const store = useAppStore.getState();
      if (store.openFiles.find((f: OpenFile) => f.path === deleteTarget.path)) {
        store.closeFile(deleteTarget.path);
      }
      await refreshTree();
    } catch (err) {
      console.error('Delete failed:', err);
    }
    setDeleteTarget(null);
  }, [deleteTarget, refreshTree]);

  // Toolbar new file/folder at project root
  const handleNewFileRoot = useCallback(async () => {
    if (!projectPath) return;
    setInlineInput({ parentPath: projectPath, type: 'file', initialValue: '' });
  }, [projectPath]);

  const handleNewFolderRoot = useCallback(async () => {
    if (!projectPath) return;
    setInlineInput({ parentPath: projectPath, type: 'folder', initialValue: '' });
  }, [projectPath]);

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
    <div className="file-explorer" onContextMenu={handleRootContextMenu}>
      {/* Toolbar */}
      <div className="explorer-toolbar">
        <span className="explorer-project-name" title={projectPath}>{projectName}</span>
        <div className="explorer-toolbar-actions">
          <button className="explorer-toolbar-btn" title="New File" onClick={handleNewFileRoot}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M11.5 1H4.5L3 2.5v11l1.5 1.5h7l1.5-1.5v-11L11.5 1zM12 13.5l-.5.5h-7l-.5-.5v-11l.5-.5h7l.5.5v11z"/>
              <path d="M8 4v3H5v1h3v3h1V8h3V7H9V4H8z"/>
            </svg>
          </button>
          <button className="explorer-toolbar-btn" title="New Folder" onClick={handleNewFolderRoot}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M14 4H9.618l-1-2H2a1 1 0 00-1 1v10a1 1 0 001 1h12a1 1 0 001-1V5a1 1 0 00-1-1zm0 9H2V3h6.382l1 2H14v8z"/>
              <path d="M8 6v2H6v1h2v2h1V9h2V8H9V6H8z"/>
            </svg>
          </button>
          <button className="explorer-toolbar-btn" title="Collapse All" onClick={collapseAll}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M9 9H4v1h5V9z"/>
              <path d="M7 1L1 5v6l6 4 6-4V5L7 1zm0 1.15l4.64 3.1L7 8.35 2.36 5.25 7 2.15zM2 5.96l4.5 3V13L2 10.54V5.96zm5.5 7.04V8.96L12 5.96v4.58L7.5 13z"/>
            </svg>
          </button>
          <button className="explorer-toolbar-btn" title="Refresh" onClick={refreshTree}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M13.451 5.609l-.579-.939-1.068.812-.076.094c-.335.415-.927 1.146-1.545 1.146-.277 0-.588-.1-.926-.296a2.747 2.747 0 01-.669-.574 1.478 1.478 0 00-.081-.088l-.591-.538-.088-.079-.09-.051A3.442 3.442 0 005.021 4C3.353 4 2 5.353 2 7.021s1.353 3.021 3.021 3.021a3.44 3.44 0 002.563-1.145l-.74-.671A2.448 2.448 0 015.02 9.042 2.022 2.022 0 013 7.021C3 5.905 3.905 5 5.021 5c.55 0 1.072.248 1.428.656l.003.003.585.532a3.75 3.75 0 00.915.787c.54.314 1.072.47 1.583.47.885 0 1.577-.424 2.074-.939l1.842 2.982.849-.524-1.846-2.984.997-.374z"/>
            </svg>
          </button>
        </div>
      </div>

      {/* File tree */}
      <div className="sidebar-content">
        {/* Root-level inline input */}
        {inlineInput && (inlineInput.type === 'file' || inlineInput.type === 'folder') && inlineInput.parentPath === projectPath && (
          <InlineNewInput
            depth={0}
            type={inlineInput.type}
            onSubmit={handleInlineSubmit}
            onCancel={handleInlineCancel}
          />
        )}
        {fileTree.map((entry: FileEntry) => (
          <FileTreeItem
            key={entry.path}
            entry={entry}
            depth={0}
            onContextMenu={handleContextMenu}
            inlineInput={inlineInput}
            onInlineSubmit={handleInlineSubmit}
            onInlineCancel={handleInlineCancel}
          />
        ))}
      </div>

      {/* Context Menu */}
      {contextMenu && (
        <ContextMenu
          state={contextMenu}
          onClose={() => setContextMenu(null)}
          onAction={handleContextAction}
        />
      )}

      {/* Delete Confirmation */}
      {deleteTarget && (
        <DeleteDialog
          entry={deleteTarget}
          onConfirm={handleDelete}
          onCancel={() => setDeleteTarget(null)}
        />
      )}
    </div>
  );
}
