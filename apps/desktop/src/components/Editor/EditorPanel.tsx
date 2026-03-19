import { useRef, useCallback } from 'react';
import type { editor } from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';
import EditorTabs from './EditorTabs';
import MonacoEditor from './MonacoEditor';
import FindReplace from './FindReplace';
import SplitEditor from './SplitEditor';
import BlameView from '../Git/BlameView';
import Breadcrumbs from './Breadcrumbs';
import MarkdownPreview from './MarkdownPreview';
import { useAppStore } from '../../stores/appStore';

const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico', 'bmp'];

function isImageFile(path: string): boolean {
  const ext = path.split('.').pop()?.toLowerCase() || '';
  return IMAGE_EXTENSIONS.includes(ext);
}

function isMarkdownFile(path: string): boolean {
  const ext = path.split('.').pop()?.toLowerCase() || '';
  return ext === 'md' || ext === 'mdx';
}

function isRunnableFile(path: string): boolean {
  const ext = path.split('.').pop()?.toLowerCase() || '';
  return ['py', 'js', 'ts', 'rs', 'go', 'java', 'c', 'cpp', 'rb', 'php', 'sh', 'bash', 'lua', 'swift', 'kt'].includes(ext);
}

function getRunCommand(filePath: string, projectPath: string | null): string | null {
  const ext = filePath.split('.').pop()?.toLowerCase() || '';
  const quotedPath = `"${filePath}"`;

  switch (ext) {
    case 'py': return `python3 ${quotedPath}`;
    case 'js': return `node ${quotedPath}`;
    case 'ts': return `npx tsx ${quotedPath}`;
    case 'rs': {
      // If inside a Cargo project, use cargo run; otherwise compile & run
      if (projectPath) return `cd "${projectPath}" && cargo run`;
      return `rustc ${quotedPath} -o /tmp/rs_out && /tmp/rs_out`;
    }
    case 'go': return `go run ${quotedPath}`;
    case 'java': {
      const className = filePath.split('/').pop()?.replace('.java', '') || 'Main';
      const dir = filePath.substring(0, filePath.lastIndexOf('/'));
      return `cd "${dir}" && javac "${filePath.split('/').pop()}" && java ${className}`;
    }
    case 'c': return `gcc ${quotedPath} -o /tmp/c_out && /tmp/c_out`;
    case 'cpp': return `g++ ${quotedPath} -o /tmp/cpp_out && /tmp/cpp_out`;
    case 'rb': return `ruby ${quotedPath}`;
    case 'php': return `php ${quotedPath}`;
    case 'sh':
    case 'bash': return `bash ${quotedPath}`;
    case 'lua': return `lua ${quotedPath}`;
    case 'swift': return `swift ${quotedPath}`;
    case 'kt': return `kotlinc ${quotedPath} -include-runtime -d /tmp/kt_out.jar && java -jar /tmp/kt_out.jar`;
    default: return null;
  }
}

export default function EditorPanel() {
  const { activeFile, openFiles, projectPath, showFindReplace, setShowFindReplace, showBlameView, blameFilePath, splitEditorMode, splitEditorRightPath, markdownPreviewVisible, toggleMarkdownPreview } = useAppStore();
  const activeFileData = openFiles.find((f) => f.path === activeFile);
  const editorInstanceRef = useRef<editor.IStandaloneCodeEditor | null>(null);

  const handleEditorMount = useCallback((instance: editor.IStandaloneCodeEditor) => {
    editorInstanceRef.current = instance;
  }, []);

  const handleRunFile = useCallback(async () => {
    if (!activeFileData) return;
    const cmd = getRunCommand(activeFileData.path, projectPath);
    if (!cmd) return;

    // Ensure terminal is visible
    const store = useAppStore.getState();
    if (!store.terminalVisible) store.toggleTerminal();
    store.setBottomPanelTab('terminal');

    // Send command to terminal
    try {
      await invoke('write_terminal', { id: 'main', data: cmd + '\n' });
    } catch {
      // Terminal might not be spawned yet — try spawning first
      try {
        await invoke('spawn_terminal', { id: 'main', rows: 24, cols: 80 });
        // Wait a moment for terminal to start
        await new Promise((r) => setTimeout(r, 300));
        await invoke('write_terminal', { id: 'main', data: cmd + '\n' });
      } catch (err) {
        console.error('Failed to run file:', err);
      }
    }
  }, [activeFileData, projectPath]);

  // Blame view replaces editor content
  if (showBlameView && blameFilePath && projectPath) {
    return (
      <div className="editor-panel">
        <EditorTabs />
        <BlameView filePath={blameFilePath} projectPath={projectPath} />
      </div>
    );
  }

  // Split editor mode
  if (splitEditorMode !== 'off' && activeFile && splitEditorRightPath) {
    return (
      <div className="editor-panel">
        <EditorTabs />
        <div className="editor-content">
          <SplitEditor leftPath={activeFile} rightPath={splitEditorRightPath} />
        </div>
      </div>
    );
  }

  // Image preview
  if (activeFileData && isImageFile(activeFileData.path)) {
    return (
      <div className="editor-panel">
        <EditorTabs />
        <div className="image-preview">
          <img
            src={`https://asset.localhost/${activeFileData.path}`}
            alt={activeFileData.name}
            onError={(e) => {
              // Fallback: try file:// protocol
              (e.target as HTMLImageElement).src = `file://${activeFileData.path}`;
            }}
          />
          <div className="image-preview-info">{activeFileData.name}</div>
        </div>
      </div>
    );
  }

  const showMdPreview = activeFileData && isMarkdownFile(activeFileData.path) && markdownPreviewVisible;

  return (
    <div className="editor-panel">
      <EditorTabs />
      {activeFileData && (
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <Breadcrumbs filePath={activeFileData.path} projectRoot={projectPath || undefined} />
          {isRunnableFile(activeFileData.path) && (
            <button
              onClick={handleRunFile}
              style={{
                background: 'var(--accent-green, #28a745)',
                border: 'none',
                borderRadius: 3,
                color: '#fff',
                padding: '2px 8px',
                cursor: 'pointer',
                fontSize: 11,
                marginRight: 4,
                flexShrink: 0,
                display: 'flex',
                alignItems: 'center',
                gap: 4,
              }}
              title={`Run ${activeFileData.name}`}
            >
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
                <path d="M3 2l10 6-10 6V2z" />
              </svg>
              Run
            </button>
          )}
          {isMarkdownFile(activeFileData.path) && (
            <button
              onClick={toggleMarkdownPreview}
              style={{
                background: markdownPreviewVisible ? 'var(--accent)' : 'none',
                border: '1px solid var(--border-color)',
                borderRadius: 3,
                color: markdownPreviewVisible ? '#fff' : 'var(--text-secondary)',
                padding: '2px 8px',
                cursor: 'pointer',
                fontSize: 11,
                marginRight: 8,
                flexShrink: 0,
              }}
              title="Toggle Markdown Preview"
            >
              Preview
            </button>
          )}
        </div>
      )}
      <div className="editor-content" style={showMdPreview ? { display: 'flex' } : undefined}>
        <div style={showMdPreview ? { flex: 1, overflow: 'hidden' } : { height: '100%' }}>
          <FindReplace
            editorInstance={editorInstanceRef.current}
            visible={showFindReplace && !!activeFileData}
            onClose={() => setShowFindReplace(false)}
          />
          <MonacoEditor onEditorMount={handleEditorMount} />
        </div>
        {showMdPreview && activeFileData && (
          <div style={{ flex: 1, overflow: 'auto', borderLeft: '1px solid var(--border-color)' }}>
            <MarkdownPreview content={activeFileData.content} />
          </div>
        )}
      </div>
    </div>
  );
}
