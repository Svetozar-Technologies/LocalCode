import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';

interface GitCommit {
  hash: string;
  shortHash: string;
  message: string;
  author: string;
  email: string;
  date: string;
  relativeDate: string;
}

const styles = {
  container: {
    display: 'flex',
    flexDirection: 'column' as const,
    height: '100%',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  searchBar: {
    padding: '8px 12px',
    borderBottom: '1px solid var(--border-color)',
  } as React.CSSProperties,
  searchInput: {
    width: '100%',
    background: 'var(--border-color)',
    border: '1px solid var(--border-color)',
    borderRadius: 3,
    color: 'var(--text-primary)',
    padding: '5px 8px',
    fontSize: 12,
    outline: 'none',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  commitList: {
    flex: 1,
    overflow: 'auto',
  } as React.CSSProperties,
  commitItem: {
    display: 'flex',
    flexDirection: 'column' as const,
    padding: '8px 12px',
    borderBottom: '1px solid var(--bg-tertiary)',
    cursor: 'pointer',
    transition: 'background 0.05s',
    gap: 4,
  } as React.CSSProperties,
  commitItemHover: {
    background: 'var(--bg-hover)',
  } as React.CSSProperties,
  commitItemSelected: {
    background: '#062f4a',
    borderLeft: '2px solid #007acc',
  } as React.CSSProperties,
  commitRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
  } as React.CSSProperties,
  hash: {
    fontSize: 11,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
    color: '#569cd6',
    flexShrink: 0,
  } as React.CSSProperties,
  message: {
    fontSize: 13,
    color: 'var(--text-primary)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
    flex: 1,
  } as React.CSSProperties,
  metaRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    fontSize: 11,
    color: 'var(--text-muted)',
  } as React.CSSProperties,
  author: {
    color: 'var(--text-secondary)',
    fontWeight: 500,
  } as React.CSSProperties,
  date: {
    marginLeft: 'auto',
    flexShrink: 0,
  } as React.CSSProperties,
  loadMoreButton: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '10px 12px',
    background: 'none',
    border: 'none',
    color: '#007acc',
    cursor: 'pointer',
    fontSize: 12,
    width: '100%',
  } as React.CSSProperties,
  loading: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 20,
    color: 'var(--text-secondary)',
    fontSize: 12,
  } as React.CSSProperties,
  empty: {
    display: 'flex',
    flexDirection: 'column' as const,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 32,
    color: 'var(--text-muted)',
    fontSize: 13,
    gap: 8,
  } as React.CSSProperties,
  graph: {
    width: 20,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
  } as React.CSSProperties,
  dot: {
    width: 8,
    height: 8,
    borderRadius: '50%',
    background: '#007acc',
    border: '2px solid var(--bg-secondary)',
  } as React.CSSProperties,
  line: {
    width: 2,
    background: 'var(--border-color)',
    position: 'absolute' as const,
    top: 0,
    bottom: 0,
    left: 9,
  } as React.CSSProperties,
};

interface HistoryViewProps {
  fileFilter?: string | null;
}

export default function HistoryView({ fileFilter }: HistoryViewProps = {}) {
  const { projectPath } = useAppStore();
  const [commits, setCommits] = useState<GitCommit[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [page, setPage] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const PAGE_SIZE = 50;

  const fetchCommits = useCallback(
    async (pageNum: number, append = false) => {
      if (!projectPath) return;
      setLoading(true);

      try {
        const result = await invoke<GitCommit[]>(fileFilter ? 'git_file_log' : 'git_log', {
          path: projectPath,
          ...(fileFilter
            ? { filePath: fileFilter, count: PAGE_SIZE }
            : { skip: pageNum * PAGE_SIZE, limit: PAGE_SIZE, search: searchQuery || undefined }),
        });

        if (append) {
          setCommits((prev) => [...prev, ...result]);
        } else {
          setCommits(result);
        }

        setHasMore(result.length >= PAGE_SIZE);
      } catch (err) {
        console.error('Failed to fetch git log:', err);
        if (!append) setCommits([]);
      }

      setLoading(false);
    },
    [projectPath, searchQuery, fileFilter]
  );

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setPage(0);
    fetchCommits(0);
  }, [fetchCommits]);

  const handleLoadMore = useCallback(() => {
    const nextPage = page + 1;
    setPage(nextPage);
    fetchCommits(nextPage, true);
  }, [page, fetchCommits]);

  const handleSearchKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      setPage(0);
      fetchCommits(0);
    }
  };

  const handleCopyHash = useCallback((hash: string) => {
    navigator.clipboard.writeText(hash).catch(() => {});
  }, []);

  const filteredCommits = commits;

  if (!projectPath) {
    return (
      <div style={styles.empty}>
        <span>Open a folder to view commit history</span>
      </div>
    );
  }

  return (
    <div style={styles.container}>
      {fileFilter && (
        <div style={{ padding: '6px 12px', fontSize: 11, color: 'var(--accent)', background: 'rgba(0,122,204,0.08)', borderBottom: '1px solid var(--border-color)' }}>
          History for: {fileFilter.split('/').pop()}
        </div>
      )}
      <div style={styles.searchBar}>
        <input
          style={styles.searchInput}
          type="text"
          placeholder="Search commits..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={handleSearchKeyDown}
          onFocus={(e) => { (e.target as HTMLInputElement).style.borderColor = '#007acc'; }}
          onBlur={(e) => { (e.target as HTMLInputElement).style.borderColor = 'var(--border-color)'; }}
        />
      </div>

      <div style={styles.commitList}>
        {loading && commits.length === 0 && (
          <div style={styles.loading}>Loading commit history...</div>
        )}

        {!loading && commits.length === 0 && (
          <div style={styles.empty}>
            <svg width="24" height="24" viewBox="0 0 16 16" fill="var(--text-muted)">
              <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 13A6 6 0 118 2a6 6 0 010 12zm0-9.5a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 018 4.5zm0 7a.75.75 0 100-1.5.75.75 0 000 1.5z" />
            </svg>
            <span>No commits found</span>
          </div>
        )}

        {filteredCommits.map((commit, index) => (
          <div
            key={commit.hash}
            style={{
              ...styles.commitItem,
              ...(selectedHash === commit.hash ? styles.commitItemSelected : {}),
              position: 'relative',
            }}
            onClick={() => setSelectedHash(commit.hash === selectedHash ? null : commit.hash)}
            onMouseEnter={(e) => {
              if (selectedHash !== commit.hash) {
                (e.currentTarget as HTMLElement).style.background = 'var(--bg-hover)';
              }
            }}
            onMouseLeave={(e) => {
              if (selectedHash !== commit.hash) {
                (e.currentTarget as HTMLElement).style.background = 'transparent';
              }
            }}
          >
            <div style={styles.commitRow}>
              <div style={styles.graph}>
                <div style={{
                  ...styles.dot,
                  background: index === 0 ? '#4ec9b0' : '#007acc',
                }} />
              </div>
              <span style={styles.message} title={commit.message}>
                {commit.message}
              </span>
            </div>
            <div style={{ ...styles.metaRow, paddingLeft: 28 }}>
              <span
                style={styles.hash}
                title={`Click to copy: ${commit.hash}`}
                onClick={(e) => {
                  e.stopPropagation();
                  handleCopyHash(commit.hash);
                }}
              >
                {commit.shortHash}
              </span>
              <span style={styles.author}>{commit.author}</span>
              <span style={styles.date}>{commit.relativeDate}</span>
            </div>
          </div>
        ))}

        {hasMore && !loading && commits.length > 0 && (
          <button
            style={styles.loadMoreButton}
            onClick={handleLoadMore}
            onMouseEnter={(e) => { (e.target as HTMLElement).style.background = 'var(--bg-hover)'; }}
            onMouseLeave={(e) => { (e.target as HTMLElement).style.background = 'none'; }}
          >
            Load more commits...
          </button>
        )}

        {loading && commits.length > 0 && (
          <div style={styles.loading}>Loading...</div>
        )}
      </div>
    </div>
  );
}
