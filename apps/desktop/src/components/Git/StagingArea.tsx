import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';
import type { GitFileStatus } from '../../types';

interface StagingAreaProps {
  onRefresh: () => void;
}

const styles = {
  section: {
    display: 'flex',
    flexDirection: 'column' as const,
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  sectionHeader: {
    display: 'flex',
    alignItems: 'center',
    padding: '6px 12px',
    background: 'var(--bg-tertiary)',
    borderBottom: '1px solid var(--border-color)',
    fontSize: 11,
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    letterSpacing: 0.5,
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    userSelect: 'none' as const,
    gap: 6,
  } as React.CSSProperties,
  count: {
    background: 'var(--border-color)',
    color: 'var(--text-primary)',
    borderRadius: 8,
    padding: '0 6px',
    fontSize: 10,
    fontWeight: 600,
  } as React.CSSProperties,
  stageAllButton: {
    marginLeft: 'auto',
    background: 'none',
    border: 'none',
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    padding: '2px 4px',
    borderRadius: 3,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  } as React.CSSProperties,
  fileList: {
    display: 'flex',
    flexDirection: 'column' as const,
  } as React.CSSProperties,
  fileItem: {
    display: 'flex',
    alignItems: 'center',
    padding: '4px 12px',
    cursor: 'pointer',
    fontSize: 12,
    gap: 8,
    transition: 'background 0.05s',
  } as React.CSSProperties,
  fileItemHover: {
    background: 'var(--bg-hover)',
  } as React.CSSProperties,
  fileName: {
    flex: 1,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
    color: 'var(--text-primary)',
  } as React.CSSProperties,
  statusBadge: {
    fontSize: 10,
    fontWeight: 700,
    width: 16,
    textAlign: 'center' as const,
    flexShrink: 0,
  } as React.CSSProperties,
  actionButton: {
    width: 20,
    height: 20,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'none',
    border: 'none',
    borderRadius: 3,
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    flexShrink: 0,
    opacity: 0,
    transition: 'opacity 0.1s',
  } as React.CSSProperties,
  empty: {
    padding: '12px 16px',
    color: 'var(--text-muted)',
    fontSize: 12,
    textAlign: 'center' as const,
  } as React.CSSProperties,
  chevron: {
    width: 12,
    height: 12,
    transition: 'transform 0.15s',
    flexShrink: 0,
  } as React.CSSProperties,
};

function getStatusColor(status: string): string {
  switch (status) {
    case 'modified': return '#ce9178';
    case 'added': return '#4ec9b0';
    case 'deleted': return '#f44747';
    case 'untracked': return '#4ec9b0';
    case 'renamed': return '#569cd6';
    default: return 'var(--text-primary)';
  }
}

function getStatusLetter(status: string): string {
  switch (status) {
    case 'modified': return 'M';
    case 'added': return 'A';
    case 'deleted': return 'D';
    case 'untracked': return 'U';
    case 'renamed': return 'R';
    default: return '?';
  }
}

interface FileItemProps {
  file: GitFileStatus;
  staged: boolean;
  onStageToggle: () => void;
  onOpenFile: () => void;
}

function FileItem({ file, staged, onStageToggle, onOpenFile }: FileItemProps) {
  const [hovered, setHovered] = useState(false);
  const fileName = file.path.split('/').pop() || file.path;
  const dirPath = file.path.split('/').slice(0, -1).join('/');

  return (
    <div
      style={{
        ...styles.fileItem,
        ...(hovered ? styles.fileItemHover : {}),
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onOpenFile}
    >
      <svg width="14" height="14" viewBox="0 0 16 16" fill={getStatusColor(file.status)}>
        <path d="M13.71 4.29l-3-3L10 1H4L3 2v12l1 1h9l1-1V5l-.29-.71zM13 14H4V2h5v4h4v8z" />
      </svg>
      <span style={styles.fileName} title={file.path}>
        {fileName}
        {dirPath && (
          <span style={{ color: 'var(--text-muted)', marginLeft: 6, fontSize: 11 }}>{dirPath}</span>
        )}
      </span>
      <button
        style={{
          ...styles.actionButton,
          opacity: hovered ? 1 : 0,
        }}
        onClick={(e) => {
          e.stopPropagation();
          onStageToggle();
        }}
        title={staged ? 'Unstage' : 'Stage'}
        onMouseEnter={(e) => { (e.target as HTMLElement).style.color = 'var(--text-primary)'; }}
        onMouseLeave={(e) => { (e.target as HTMLElement).style.color = 'var(--text-secondary)'; }}
      >
        {staged ? (
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 8l4-4 .7.7L5.4 8l3.3 3.3-.7.7L4 8z" />
          </svg>
        ) : (
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M14 7v1H8v6H7V8H1V7h6V1h1v6h6z" />
          </svg>
        )}
      </button>
      <span
        style={{
          ...styles.statusBadge,
          color: getStatusColor(file.status),
        }}
      >
        {getStatusLetter(file.status)}
      </span>
    </div>
  );
}

export default function StagingArea({ onRefresh }: StagingAreaProps) {
  const { projectPath, gitStatus, openFile } = useAppStore();
  const [stagedFiles, setStagedFiles] = useState<Set<string>>(new Set());
  const [unstagedExpanded, setUnstagedExpanded] = useState(true);
  const [stagedExpanded, setStagedExpanded] = useState(true);

  const unstagedFiles = gitStatus.filter((f) => !stagedFiles.has(f.path));
  const staged = gitStatus.filter((f) => stagedFiles.has(f.path));

  const handleStage = useCallback(
    async (path: string) => {
      if (!projectPath) return;
      try {
        await invoke('git_add', { path: projectPath, files: [path] });
        setStagedFiles((prev) => new Set([...prev, path]));
        onRefresh();
      } catch (err) {
        console.error('Failed to stage file:', err);
      }
    },
    [projectPath, onRefresh]
  );

  const handleUnstage = useCallback(
    async (path: string) => {
      if (!projectPath) return;
      try {
        await invoke('git_unstage', { path: projectPath, files: [path] });
        setStagedFiles((prev) => {
          const next = new Set(prev);
          next.delete(path);
          return next;
        });
        onRefresh();
      } catch (err) {
        console.error('Failed to unstage file:', err);
      }
    },
    [projectPath, onRefresh]
  );

  const handleStageAll = useCallback(async () => {
    if (!projectPath) return;
    try {
      await invoke('git_add', {
        path: projectPath,
        files: unstagedFiles.map((f) => f.path),
      });
      setStagedFiles(new Set(gitStatus.map((f) => f.path)));
      onRefresh();
    } catch (err) {
      console.error('Failed to stage all:', err);
    }
  }, [projectPath, unstagedFiles, gitStatus, onRefresh]);

  const handleUnstageAll = useCallback(async () => {
    if (!projectPath) return;
    try {
      await invoke('git_unstage', {
        path: projectPath,
        files: staged.map((f) => f.path),
      });
      setStagedFiles(new Set());
      onRefresh();
    } catch (err) {
      console.error('Failed to unstage all:', err);
    }
  }, [projectPath, staged, onRefresh]);

  const handleOpenFile = useCallback(
    async (file: GitFileStatus) => {
      if (!projectPath) return;
      try {
        const fullPath = file.path.startsWith('/') ? file.path : `${projectPath}/${file.path}`;
        const content = await invoke<string>('read_file', { path: fullPath });
        const ext = file.path.split('.').pop()?.toLowerCase() || '';
        const langMap: Record<string, string> = {
          ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact',
          py: 'python', rs: 'rust', go: 'go', java: 'java',
          html: 'html', css: 'css', json: 'json', md: 'markdown',
        };
        openFile({
          path: fullPath,
          name: file.path.split('/').pop() || file.path,
          content,
          language: langMap[ext] || 'plaintext',
          modified: false,
        });
      } catch (err) {
        console.error('Failed to open file:', err);
      }
    },
    [projectPath, openFile]
  );

  return (
    <div style={styles.section}>
      {/* Staged Changes */}
      <div
        style={styles.sectionHeader}
        onClick={() => setStagedExpanded(!stagedExpanded)}
      >
        <svg
          style={{
            ...styles.chevron,
            transform: stagedExpanded ? 'rotate(90deg)' : 'rotate(0deg)',
          }}
          viewBox="0 0 16 16"
          fill="currentColor"
        >
          <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" fill="none" />
        </svg>
        Staged Changes
        {staged.length > 0 && <span style={styles.count}>{staged.length}</span>}
        {staged.length > 0 && (
          <button
            style={styles.stageAllButton}
            onClick={(e) => {
              e.stopPropagation();
              handleUnstageAll();
            }}
            title="Unstage All"
            onMouseEnter={(e) => { (e.target as HTMLElement).style.color = 'var(--text-primary)'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.color = 'var(--text-secondary)'; }}
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 8l4-4 .7.7L5.4 8l3.3 3.3-.7.7L4 8z" />
            </svg>
          </button>
        )}
      </div>
      {stagedExpanded && (
        <div style={styles.fileList}>
          {staged.length === 0 ? (
            <div style={styles.empty}>No staged changes</div>
          ) : (
            staged.map((file) => (
              <FileItem
                key={file.path}
                file={file}
                staged={true}
                onStageToggle={() => handleUnstage(file.path)}
                onOpenFile={() => handleOpenFile(file)}
              />
            ))
          )}
        </div>
      )}

      {/* Unstaged Changes */}
      <div
        style={styles.sectionHeader}
        onClick={() => setUnstagedExpanded(!unstagedExpanded)}
      >
        <svg
          style={{
            ...styles.chevron,
            transform: unstagedExpanded ? 'rotate(90deg)' : 'rotate(0deg)',
          }}
          viewBox="0 0 16 16"
          fill="currentColor"
        >
          <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" fill="none" />
        </svg>
        Changes
        {unstagedFiles.length > 0 && <span style={styles.count}>{unstagedFiles.length}</span>}
        {unstagedFiles.length > 0 && (
          <button
            style={styles.stageAllButton}
            onClick={(e) => {
              e.stopPropagation();
              handleStageAll();
            }}
            title="Stage All"
            onMouseEnter={(e) => { (e.target as HTMLElement).style.color = 'var(--text-primary)'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.color = 'var(--text-secondary)'; }}
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M14 7v1H8v6H7V8H1V7h6V1h1v6h6z" />
            </svg>
          </button>
        )}
      </div>
      {unstagedExpanded && (
        <div style={styles.fileList}>
          {unstagedFiles.length === 0 ? (
            <div style={styles.empty}>No unstaged changes</div>
          ) : (
            unstagedFiles.map((file) => (
              <FileItem
                key={file.path}
                file={file}
                staged={false}
                onStageToggle={() => handleStage(file.path)}
                onOpenFile={() => handleOpenFile(file)}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}
