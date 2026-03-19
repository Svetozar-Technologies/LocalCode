import { useCallback, useRef, useEffect } from 'react';
import Editor, { type OnMount } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { useAppStore } from '../../stores/appStore';
import { invoke } from '@tauri-apps/api/core';
import { useInlineCompletion } from '../../hooks/useInlineCompletion';
import InlineEdit from './InlineEdit';

/** Map store theme names to Monaco theme IDs */
const THEME_MAP: Record<string, string> = {
  dark: 'localcode-dark',
  light: 'localcode-light',
  monokai: 'localcode-monokai',
  solarized: 'localcode-solarized',
};

/** CSS variable overrides per theme (applied to :root) */
const CSS_THEMES: Record<string, Record<string, string>> = {
  dark: {
    '--bg-primary': '#1e1e1e',
    '--bg-secondary': '#252526',
    '--bg-tertiary': '#2d2d2d',
    '--bg-hover': '#2a2d2e',
    '--bg-active': '#37373d',
    '--bg-input': '#3c3c3c',
    '--border-color': '#3c3c3c',
    '--text-primary': '#cccccc',
    '--text-secondary': '#969696',
    '--text-muted': '#6a6a6a',
    '--text-bright': '#e7e7e7',
    '--tab-active-bg': '#1e1e1e',
    '--tab-inactive-bg': '#2d2d2d',
  },
  light: {
    '--bg-primary': '#ffffff',
    '--bg-secondary': '#f3f3f3',
    '--bg-tertiary': '#ececec',
    '--bg-hover': '#e8e8e8',
    '--bg-active': '#d6d6d6',
    '--bg-input': '#ffffff',
    '--border-color': '#cccccc',
    '--text-primary': '#1e1e1e',
    '--text-secondary': '#616161',
    '--text-muted': '#999999',
    '--text-bright': '#000000',
    '--tab-active-bg': '#ffffff',
    '--tab-inactive-bg': '#ececec',
  },
  monokai: {
    '--bg-primary': '#272822',
    '--bg-secondary': '#1e1f1c',
    '--bg-tertiary': '#333429',
    '--bg-hover': '#3e3d32',
    '--bg-active': '#49483e',
    '--bg-input': '#3e3d32',
    '--border-color': '#49483e',
    '--text-primary': '#f8f8f2',
    '--text-secondary': '#a6a69c',
    '--text-muted': '#75715e',
    '--text-bright': '#ffffff',
    '--tab-active-bg': '#272822',
    '--tab-inactive-bg': '#1e1f1c',
  },
  solarized: {
    '--bg-primary': '#002b36',
    '--bg-secondary': '#073642',
    '--bg-tertiary': '#0a4050',
    '--bg-hover': '#0d4a5a',
    '--bg-active': '#1a5a6a',
    '--bg-input': '#073642',
    '--border-color': '#2aa198',
    '--text-primary': '#839496',
    '--text-secondary': '#657b83',
    '--text-muted': '#586e75',
    '--text-bright': '#fdf6e3',
    '--tab-active-bg': '#002b36',
    '--tab-inactive-bg': '#073642',
  },
};

function applyCSS(themeName: string) {
  const vars = CSS_THEMES[themeName] || CSS_THEMES.dark;
  const root = document.documentElement;
  for (const [key, value] of Object.entries(vars)) {
    root.style.setProperty(key, value);
  }
}

let themesRegistered = false;

function registerAllThemes(monaco: any) {
  if (themesRegistered) return;
  themesRegistered = true;

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

  monaco.editor.defineTheme('localcode-light', {
    base: 'vs',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '008000' },
      { token: 'keyword', foreground: '0000FF' },
      { token: 'string', foreground: 'A31515' },
      { token: 'number', foreground: '098658' },
      { token: 'type', foreground: '267f99' },
      { token: 'function', foreground: '795E26' },
      { token: 'variable', foreground: '001080' },
    ],
    colors: {
      'editor.background': '#ffffff',
      'editor.foreground': '#1e1e1e',
      'editor.lineHighlightBackground': '#f5f5f5',
      'editor.selectionBackground': '#add6ff',
      'editorCursor.foreground': '#000000',
      'editorWhitespace.foreground': '#d4d4d4',
    },
  });

  monaco.editor.defineTheme('localcode-monokai', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '75715E' },
      { token: 'keyword', foreground: 'F92672' },
      { token: 'string', foreground: 'E6DB74' },
      { token: 'number', foreground: 'AE81FF' },
      { token: 'type', foreground: '66D9EF', fontStyle: 'italic' },
      { token: 'function', foreground: 'A6E22E' },
      { token: 'variable', foreground: 'F8F8F2' },
    ],
    colors: {
      'editor.background': '#272822',
      'editor.foreground': '#F8F8F2',
      'editor.lineHighlightBackground': '#3e3d32',
      'editor.selectionBackground': '#49483e',
      'editorCursor.foreground': '#F8F8F0',
      'editorWhitespace.foreground': '#464741',
    },
  });

  monaco.editor.defineTheme('localcode-solarized', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '586E75' },
      { token: 'keyword', foreground: '859900' },
      { token: 'string', foreground: '2AA198' },
      { token: 'number', foreground: 'D33682' },
      { token: 'type', foreground: 'B58900' },
      { token: 'function', foreground: '268BD2' },
      { token: 'variable', foreground: '839496' },
    ],
    colors: {
      'editor.background': '#002b36',
      'editor.foreground': '#839496',
      'editor.lineHighlightBackground': '#073642',
      'editor.selectionBackground': '#073642',
      'editorCursor.foreground': '#839496',
      'editorWhitespace.foreground': '#073642',
    },
  });
}

export default function MonacoEditor() {
  const { openFiles, activeFile, updateFileContent, markFileSaved, theme } = useAppStore();
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const modelsRef = useRef<Map<string, editor.ITextModel>>(new Map());

  const activeFileData = openFiles.find((f) => f.path === activeFile);
  const monacoTheme = THEME_MAP[theme] || 'localcode-dark';

  // Wire up inline completion
  useInlineCompletion(editorRef.current);

  const handleEditorMount: OnMount = useCallback((editor, monaco) => {
    editorRef.current = editor;
    registerAllThemes(monaco);
    monaco.editor.setTheme(monacoTheme);
    applyCSS(theme);

    // Track selection changes for @selection mention
    editor.onDidChangeCursorSelection(() => {
      const sel = editor.getSelection();
      if (sel && !sel.isEmpty()) {
        const selectedText = editor.getModel()?.getValueInRange(sel) || '';
        useAppStore.getState().setEditorSelection(selectedText);
      } else {
        useAppStore.getState().setEditorSelection('');
      }
    });

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
  }, [markFileSaved, monacoTheme, theme]);

  // React to theme changes
  useEffect(() => {
    const monaco = (window as any).monaco;
    if (monaco) {
      registerAllThemes(monaco);
      monaco.editor.setTheme(monacoTheme);
    }
    applyCSS(theme);
  }, [theme, monacoTheme]);

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
          <div className="shortcut"><kbd>Cmd+P</kbd> Quick Open File</div>
          <div className="shortcut"><kbd>Cmd+K</kbd> Inline Edit Selection</div>
          <div className="shortcut"><kbd>Cmd+F</kbd> Find in File</div>
          <div className="shortcut"><kbd>Cmd+Shift+F</kbd> Search in Files</div>
          <div className="shortcut"><kbd>Cmd+`</kbd> Toggle Terminal</div>
          <div className="shortcut"><kbd>Cmd+I</kbd> AI Chat</div>
          <div className="shortcut"><kbd>Cmd+B</kbd> Toggle Sidebar</div>
          <div className="shortcut"><kbd>Cmd+Shift+I</kbd> Composer</div>
        </div>
      </div>
    );
  }

  return (
    <div style={{ position: 'relative', height: '100%' }}>
      <InlineEdit editorInstance={editorRef.current} />
      <Editor
        height="100%"
        defaultLanguage={activeFileData.language}
        defaultValue={activeFileData.content}
        theme={monacoTheme}
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
          inlineSuggest: { enabled: true },
        }}
      />
    </div>
  );
}
