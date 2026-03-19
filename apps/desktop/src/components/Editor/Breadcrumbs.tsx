import { useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';

interface BreadcrumbsProps {
  filePath: string;
  projectRoot?: string;
}

const styles = {
  container: {
    display: 'flex',
    alignItems: 'center',
    padding: '0 12px',
    height: 26,
    minHeight: 26,
    background: 'var(--bg-primary)',
    borderBottom: '1px solid var(--border-color)',
    overflow: 'hidden',
    fontSize: 12,
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
  scrollArea: {
    display: 'flex',
    alignItems: 'center',
    overflow: 'hidden',
    whiteSpace: 'nowrap' as const,
    flex: 1,
  } as React.CSSProperties,
  segment: {
    display: 'inline-flex',
    alignItems: 'center',
    color: 'var(--text-secondary)',
    cursor: 'pointer',
    padding: '2px 4px',
    borderRadius: 3,
    transition: 'color 0.1s, background 0.1s',
    whiteSpace: 'nowrap' as const,
    flexShrink: 0,
  } as React.CSSProperties,
  segmentHover: {
    color: 'var(--text-primary)',
    background: 'var(--bg-hover)',
  } as React.CSSProperties,
  separator: {
    color: 'var(--text-muted)',
    margin: '0 2px',
    fontSize: 11,
    flexShrink: 0,
    userSelect: 'none' as const,
  } as React.CSSProperties,
  fileSegment: {
    color: 'var(--text-primary)',
    fontWeight: 500,
  } as React.CSSProperties,
  icon: {
    width: 14,
    height: 14,
    marginRight: 4,
    flexShrink: 0,
  } as React.CSSProperties,
};

interface Segment {
  name: string;
  path: string;
  isDir: boolean;
  isLast: boolean;
}

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact',
    py: 'python', rs: 'rust', go: 'go', java: 'java', c: 'c', cpp: 'cpp', h: 'c',
    html: 'html', css: 'css', scss: 'scss', json: 'json', md: 'markdown',
    yml: 'yaml', yaml: 'yaml', toml: 'toml', sh: 'shell', sql: 'sql',
  };
  return map[ext] || 'plaintext';
}

export default function Breadcrumbs({ filePath, projectRoot }: BreadcrumbsProps) {
  const { openFile } = useAppStore();

  const segments = useMemo<Segment[]>(() => {
    const relativePath = projectRoot && filePath.startsWith(projectRoot)
      ? filePath.slice(projectRoot.length + 1)
      : filePath;

    const parts = relativePath.split('/').filter(Boolean);
    const result: Segment[] = [];
    let currentPath = projectRoot || '';

    for (let i = 0; i < parts.length; i++) {
      currentPath = currentPath ? `${currentPath}/${parts[i]}` : parts[i];
      result.push({
        name: parts[i],
        path: currentPath,
        isDir: i < parts.length - 1,
        isLast: i === parts.length - 1,
      });
    }

    return result;
  }, [filePath, projectRoot]);

  const handleSegmentClick = useCallback(
    async (segment: Segment) => {
      if (segment.isDir) {
        // Navigate to directory in file explorer - expand it
        const store = useAppStore.getState();
        store.toggleDir(segment.path);
      } else {
        // Open the file
        try {
          const content = await invoke<string>('read_file', { path: segment.path });
          openFile({
            path: segment.path,
            name: segment.name,
            content,
            language: getLanguageFromPath(segment.path),
            modified: false,
          });
        } catch (err) {
          console.error('Failed to open file from breadcrumb:', err);
        }
      }
    },
    [openFile]
  );

  if (segments.length === 0) return null;

  return (
    <div style={styles.container}>
      <div style={styles.scrollArea}>
        {segments.map((segment, index) => (
          <span key={segment.path} style={{ display: 'inline-flex', alignItems: 'center' }}>
            {index > 0 && (
              <span style={styles.separator}>
                <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" fill="none" />
                </svg>
              </span>
            )}
            <span
              style={{
                ...styles.segment,
                ...(segment.isLast ? styles.fileSegment : {}),
              }}
              onClick={() => handleSegmentClick(segment)}
              onMouseEnter={(e) => {
                if (!segment.isLast) {
                  Object.assign((e.target as HTMLElement).style, styles.segmentHover);
                }
              }}
              onMouseLeave={(e) => {
                if (!segment.isLast) {
                  (e.target as HTMLElement).style.color = 'var(--text-secondary)';
                  (e.target as HTMLElement).style.background = 'transparent';
                }
              }}
            >
              {segment.isDir ? (
                <svg style={styles.icon} viewBox="0 0 16 16" fill="#dcdcaa">
                  <path d="M14.5 3H7.71l-.85-.85L6.51 2h-5l-.5.5v11l.5.5h13l.5-.5v-10L14.5 3zm-.51 8.49V13h-12V3h4.29l.85.85.36.15H14v7.49z" />
                </svg>
              ) : (
                <svg style={styles.icon} viewBox="0 0 16 16" fill="var(--text-primary)">
                  <path d="M13.71 4.29l-3-3L10 1H4L3 2v12l1 1h9l1-1V5l-.29-.71zM13 14H4V2h5v4h4v8z" />
                </svg>
              )}
              {segment.name}
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}
