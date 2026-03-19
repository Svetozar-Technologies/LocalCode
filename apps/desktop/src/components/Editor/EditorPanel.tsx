import { useRef, useCallback } from 'react';
import type { editor } from 'monaco-editor';
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

export default function EditorPanel() {
  const { activeFile, openFiles, projectPath, showFindReplace, setShowFindReplace, showBlameView, blameFilePath, splitEditorMode, splitEditorRightPath, markdownPreviewVisible, toggleMarkdownPreview } = useAppStore();
  const activeFileData = openFiles.find((f) => f.path === activeFile);
  const editorInstanceRef = useRef<editor.IStandaloneCodeEditor | null>(null);

  const handleEditorMount = useCallback((instance: editor.IStandaloneCodeEditor) => {
    editorInstanceRef.current = instance;
  }, []);

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
