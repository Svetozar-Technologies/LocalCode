import { useCallback, useRef, useEffect } from 'react';
import Editor, { type OnMount } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { useAppStore } from '../../stores/appStore';
import { invoke } from '@tauri-apps/api/core';

export default function MonacoEditor() {
  const { openFiles, activeFile, updateFileContent, markFileSaved } = useAppStore();
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const modelsRef = useRef<Map<string, editor.ITextModel>>(new Map());

  const activeFileData = openFiles.find((f) => f.path === activeFile);

  const handleEditorMount: OnMount = useCallback((editor, monaco) => {
    editorRef.current = editor;

    // Configure theme
    monaco.editor.defineTheme('localcode-dark', {
      base: 'vs-dark',
      inherit: true,
      rules: [
        { token: 'comment', foreground: '6A9955' },
        { token: 'keyword', foreground: '569CD6' },
        { token: 'string', foreground: 'CE9178' },
        { token: 'number', foreground: 'B5CEA8' },
        { token: 'type', foreground: '4EC9B0' },
        { token: 'function', foreground: 'DCDCAA' },
        { token: 'variable', foreground: '9CDCFE' },
      ],
      colors: {
        'editor.background': '#1e1e1e',
        'editor.foreground': '#d4d4d4',
        'editor.lineHighlightBackground': '#2a2d2e',
        'editor.selectionBackground': '#264f78',
        'editorCursor.foreground': '#aeafad',
        'editorWhitespace.foreground': '#3b3b3b',
      },
    });
    monaco.editor.setTheme('localcode-dark');

    // Cmd+S to save
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, async () => {
      const model = editor.getModel();
      if (!model) return;
      const state = useAppStore.getState();
      const file = state.openFiles.find((f) => f.path === state.activeFile);
      if (!file) return;
      try {
        await invoke('write_file', { path: file.path, content: model.getValue() });
        markFileSaved(file.path);
      } catch (err) {
        console.error('Save failed:', err);
      }
    });

    // Cmd+P for quick open (placeholder)
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyP, () => {
      // TODO: Quick file open
    });
  }, [markFileSaved]);

  // Switch models when active file changes
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor || !activeFileData) return;

    const monaco = (window as any).monaco;
    if (!monaco) return;

    let model = modelsRef.current.get(activeFileData.path) ?? null;
    if (!model) {
      const uri = monaco.Uri.parse(`file://${activeFileData.path}`);
      model = monaco.editor.getModel(uri) ??
        monaco.editor.createModel(activeFileData.content, activeFileData.language, uri);
      if (model) modelsRef.current.set(activeFileData.path, model);
    }

    if (model && editor.getModel() !== model) {
      editor.setModel(model);
    }
  }, [activeFileData]);

  if (!activeFileData) {
    return (
      <div className="welcome-screen">
        <h1>LocalCode</h1>
        <p>Privacy-first AI-powered code editor</p>
        <div style={{ marginTop: 20, display: 'flex', flexDirection: 'column', gap: 8 }}>
          <div className="shortcut"><kbd>Cmd+O</kbd> Open Folder</div>
          <div className="shortcut"><kbd>Cmd+P</kbd> Quick Open File</div>
          <div className="shortcut"><kbd>Cmd+Shift+F</kbd> Search in Files</div>
          <div className="shortcut"><kbd>Cmd+`</kbd> Toggle Terminal</div>
          <div className="shortcut"><kbd>Cmd+I</kbd> AI Chat</div>
        </div>
      </div>
    );
  }

  return (
    <Editor
      height="100%"
      defaultLanguage={activeFileData.language}
      defaultValue={activeFileData.content}
      theme="localcode-dark"
      onMount={handleEditorMount}
      onChange={(value) => {
        if (value !== undefined && activeFile) {
          updateFileContent(activeFile, value);
        }
      }}
      options={{
        fontSize: 14,
        fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', monospace",
        fontLigatures: true,
        minimap: { enabled: true, scale: 1 },
        scrollBeyondLastLine: false,
        renderWhitespace: 'selection',
        bracketPairColorization: { enabled: true },
        guides: { bracketPairs: true, indentation: true },
        smoothScrolling: true,
        cursorBlinking: 'smooth',
        cursorSmoothCaretAnimation: 'on',
        padding: { top: 8 },
        suggest: { preview: true },
        parameterHints: { enabled: true },
        tabSize: 2,
        wordWrap: 'off',
        automaticLayout: true,
      }}
    />
  );
}
