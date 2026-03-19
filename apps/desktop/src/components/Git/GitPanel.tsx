import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';
import StagingArea from './StagingArea';
import CommitView from './CommitView';
import HistoryView from './HistoryView';

type GitTab = 'changes' | 'history';

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    height: '100%',
    background: '#252526',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  header: {
    display: 'flex',
    alignItems: 'center',
    padding: '8px 12px',
    borderBottom: '1px solid #3c3c3c',
    gap: 8,
  } as React.CSSProperties,
  branchInfo: {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    fontSize: 12,
    color: '#cccccc',
  } as React.CSSProperties,
  branchIcon: {
    color: '#569cd6',
  } as React.CSSProperties,
  branchName: {
    fontWeight: 500,
  } as React.CSSProperties,
  refreshButton: {
    marginLeft: 'auto',
    background: 'none',
    border: 'none',
    color: '#969696',
    cursor: 'pointer',
    width: 24,
    height: 24,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: 3,
  } as React.CSSProperties,
  tabs: {
    display: 'flex',
    borderBottom: '1px solid #3c3c3c',
  } as React.CSSProperties,
  tab: {
    flex: 1,
    padding: '8px 12px',
    background: 'none',
    border: 'none',
    borderBottom: '2px solid transparent',
    color: '#969696',
    cursor: 'pointer',
    fontSize: 12,
    textAlign: 'center' as const,
    transition: 'color 0.1s, border-color 0.1s',
  } as React.CSSProperties,
  tabActive: {
    color: '#cccccc',
    borderBottomColor: '#007acc',
  } as React.CSSProperties,
  content: {
    flex: 1,
    overflow: 'auto',
    display: 'flex',
    flexDirection: 'column' as const,
  } as React.CSSProperties,
  noRepo: {
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 32,
    gap: 12,
    color: '#6a6a6a',
    fontSize: 13,
    textAlign: 'center' as const,
  } as React.CSSProperties,
  initButton: {
    background: '#007acc',
    border: 'none',
    borderRadius: 4,
    color: '#ffffff',
    padding: '7px 20px',
    cursor: 'pointer',
    fontSize: 13,
    marginTop: 8,
  } as React.CSSProperties,
  statusCount: {
    background: '#007acc',
    color: '#ffffff',
    borderRadius: 8,
    padding: '0 6px',
    fontSize: 10,
    fontWeight: 600,
    marginLeft: 4,
  } as React.CSSProperties,
};

export default function GitPanel() {
  const { projectPath, gitStatus, setGitStatus, currentBranch, setCurrentBranch } = useAppStore();
  const [activeTab, setActiveTab] = useState<GitTab>('changes');
  const [loading, setLoading] = useState(false);
  const [isRepo, setIsRepo] = useState(true);

  const refreshStatus = useCallback(async () => {
    if (!projectPath) return;
    setLoading(true);
    try {
      const status = await invoke<{ path: string; status: string }[]>('git_status', {
        path: projectPath,
      });
      setGitStatus(
        status.map((s) => ({
          path: s.path,
          status: s.status as any,
        }))
      );

      const branch = await invoke<string>('git_branch', {
        path: projectPath,
      });
      setCurrentBranch(branch);
      setIsRepo(true);
    } catch (err) {
      console.error('Git status failed:', err);
      setIsRepo(false);
    }
    setLoading(false);
  }, [projectPath, setGitStatus, setCurrentBranch]);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  const handleInitRepo = useCallback(async () => {
    if (!projectPath) return;
    try {
      await invoke('git_init', { path: projectPath });
      await refreshStatus();
    } catch (err) {
      console.error('Git init failed:', err);
    }
  }, [projectPath, refreshStatus]);

  if (!projectPath) {
    return (
      <div style={styles.noRepo}>
        <svg width="32" height="32" viewBox="0 0 24 24" fill="#6a6a6a">
          <path d="M21.007 8.222A3.738 3.738 0 0 0 15.045 5.2a3.737 3.737 0 0 0 1.156 6.583 2.988 2.988 0 0 1-2.668 1.67h-2.99a4.456 4.456 0 0 0-2.989 1.165V7.4a3.737 3.737 0 1 0-1.494 0v9.117a3.776 3.776 0 1 0 1.816.099 2.99 2.99 0 0 1 2.668-1.667h2.99a4.484 4.484 0 0 0 4.223-3.039 3.736 3.736 0 0 0 3.25-3.687z" />
        </svg>
        <span>Open a folder to use Git</span>
      </div>
    );
  }

  if (!isRepo) {
    return (
      <div style={styles.noRepo}>
        <svg width="32" height="32" viewBox="0 0 24 24" fill="#6a6a6a">
          <path d="M21.007 8.222A3.738 3.738 0 0 0 15.045 5.2a3.737 3.737 0 0 0 1.156 6.583 2.988 2.988 0 0 1-2.668 1.67h-2.99a4.456 4.456 0 0 0-2.989 1.165V7.4a3.737 3.737 0 1 0-1.494 0v9.117a3.776 3.776 0 1 0 1.816.099 2.99 2.99 0 0 1 2.668-1.667h2.99a4.484 4.484 0 0 0 4.223-3.039 3.736 3.736 0 0 0 3.25-3.687z" />
        </svg>
        <span>This folder is not a Git repository.</span>
        <button
          style={styles.initButton}
          onClick={handleInitRepo}
          onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#1a8ad4'; }}
          onMouseLeave={(e) => { (e.target as HTMLElement).style.background = '#007acc'; }}
        >
          Initialize Repository
        </button>
      </div>
    );
  }

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <div style={styles.branchInfo}>
          <svg style={styles.branchIcon} width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M15 5c0-1.66-1.34-3-3-3S9 3.34 9 5c0 1.3.84 2.4 2 2.82V9c0 .55-.45 1-1 1H6c-.55 0-1-.45-1-1V7.82C6.16 7.4 7 6.3 7 5c0-1.66-1.34-3-3-3S1 3.34 1 5c0 1.3.84 2.4 2 2.82V9c0 1.1.9 2 2 2h4c1.1 0 2-.9 2-2V7.82C12.16 7.4 13 6.3 13 5z" />
          </svg>
          <span style={styles.branchName}>{currentBranch || 'main'}</span>
        </div>
        <button
          style={styles.refreshButton}
          onClick={refreshStatus}
          title="Refresh"
          onMouseEnter={(e) => { (e.target as HTMLElement).style.background = '#2a2d2e'; }}
          onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" style={loading ? { animation: 'spin 0.6s linear infinite' } : {}}>
            <path d="M13.451 5.609l-.579-.101.076-.464c-.086-.059-.173-.117-.262-.173l-.442.18-.39-.477.345-.33a6.023 6.023 0 00-.567-.637l-.455.204-.511-.375.177-.476a5.977 5.977 0 00-.79-.34l-.297.408-.585-.217-.042-.504a6.032 6.032 0 00-.85-.082l-.148.49-.598-.042-.252-.439a6.034 6.034 0 00-.824.18l.008.499-.576.148-.393-.324c-.24.145-.468.305-.683.478l.173.47-.49.304-.497-.17a5.984 5.984 0 00-.453.74l.341.376-.355.435-.548-.009a6.04 6.04 0 00-.176.82l.479.17-.17.576-.51.06a6.016 6.016 0 00.085.847l.496-.065.04.59-.442.277a5.97 5.97 0 00.347.789l.428-.22.235.544-.302.424a5.94 5.94 0 00.582.635l.37-.35.387.434-.148.496c.215.164.444.313.682.443l.286-.445.511.274.018.53a6.04 6.04 0 00.816.182l.173-.507.555.1-.101.52c.283.022.57.022.854-.002l-.102-.52.556-.098.172.508a6.01 6.01 0 00.818-.18l.016-.53.512-.276.288.443c.237-.13.465-.279.679-.443l-.15-.494.389-.432.373.348c.216-.2.42-.413.607-.64l-.304-.422.237-.543.43.22a5.97 5.97 0 00.349-.787l-.444-.278.042-.589.498.064c.07-.277.116-.558.138-.842l-.511-.06L13.451 5.609zM8 10.5c-1.38 0-2.5-1.12-2.5-2.5S6.62 5.5 8 5.5s2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z" />
          </svg>
        </button>
      </div>

      <div style={styles.tabs}>
        <button
          style={{
            ...styles.tab,
            ...(activeTab === 'changes' ? styles.tabActive : {}),
          }}
          onClick={() => setActiveTab('changes')}
          onMouseEnter={(e) => {
            if (activeTab !== 'changes') (e.target as HTMLElement).style.color = '#cccccc';
          }}
          onMouseLeave={(e) => {
            if (activeTab !== 'changes') (e.target as HTMLElement).style.color = '#969696';
          }}
        >
          Changes
          {gitStatus.length > 0 && (
            <span style={styles.statusCount}>{gitStatus.length}</span>
          )}
        </button>
        <button
          style={{
            ...styles.tab,
            ...(activeTab === 'history' ? styles.tabActive : {}),
          }}
          onClick={() => setActiveTab('history')}
          onMouseEnter={(e) => {
            if (activeTab !== 'history') (e.target as HTMLElement).style.color = '#cccccc';
          }}
          onMouseLeave={(e) => {
            if (activeTab !== 'history') (e.target as HTMLElement).style.color = '#969696';
          }}
        >
          History
        </button>
      </div>

      <div style={styles.content}>
        {activeTab === 'changes' && (
          <>
            <CommitView onRefresh={refreshStatus} />
            <StagingArea onRefresh={refreshStatus} />
          </>
        )}
        {activeTab === 'history' && <HistoryView />}
      </div>
    </div>
  );
}
