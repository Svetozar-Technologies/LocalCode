import { useEffect, useRef } from 'react';
import type { editor, Position } from 'monaco-editor';
import type { Monaco } from '@monaco-editor/react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';

const LANG_MAP: Record<string, string> = {
  typescript: 'typescript',
  typescriptreact: 'typescript',
  javascript: 'typescript',
  javascriptreact: 'typescript',
  python: 'python',
  rust: 'rust',
  go: 'go',
  c: 'c',
  cpp: 'cpp',
  java: 'java',
};

function getLanguageId(monacoLanguage: string): string | null {
  return LANG_MAP[monacoLanguage] || null;
}

export function useLspFeatures(editorInstance: editor.IStandaloneCodeEditor | null) {
  const registeredRef = useRef(false);

  useEffect(() => {
    if (!editorInstance || registeredRef.current) return;

    const monaco = (window as unknown as { monaco?: Monaco }).monaco;
    if (!monaco) return;

    registeredRef.current = true;

    // Register hover provider for all supported languages
    const languages = ['typescript', 'typescriptreact', 'javascript', 'javascriptreact', 'python', 'rust', 'go', 'c', 'cpp', 'java'];

    for (const lang of languages) {
      const lspLang = getLanguageId(lang);
      if (!lspLang) continue;

      // Hover provider
      monaco.languages.registerHoverProvider(lang, {
        provideHover: async (model: editor.ITextModel, position: Position) => {
          try {
            const filePath = model.uri.path;
            const result = await invoke<string | null>('lsp_hover', {
              filePath,
              line: position.lineNumber - 1,
              character: position.column - 1,
              language: lspLang,
            });
            if (result) {
              return {
                range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column),
                contents: [{ value: result }],
              };
            }
          } catch {
            // LSP not available — silent
          }
          return null;
        },
      });

      // Definition provider
      monaco.languages.registerDefinitionProvider(lang, {
        provideDefinition: async (model: editor.ITextModel, position: Position) => {
          try {
            const filePath = model.uri.path;
            const result = await invoke<{ uri: string; line: number; character: number } | null>('lsp_definition', {
              filePath,
              line: position.lineNumber - 1,
              character: position.column - 1,
              language: lspLang,
            });
            if (result) {
              const targetUri = monaco.Uri.parse(result.uri);
              return {
                uri: targetUri,
                range: new monaco.Range(
                  result.line + 1, result.character + 1,
                  result.line + 1, result.character + 1
                ),
              };
            }
          } catch {
            // LSP not available
          }
          return null;
        },
      });

      // References provider
      monaco.languages.registerReferenceProvider(lang, {
        provideReferences: async (model: editor.ITextModel, position: Position) => {
          try {
            const filePath = model.uri.path;
            const results = await invoke<Array<{ uri: string; line: number; character: number; end_line: number; end_character: number }>>('lsp_references', {
              filePath,
              line: position.lineNumber - 1,
              character: position.column - 1,
              language: lspLang,
            });
            return results.map((loc) => ({
              uri: monaco.Uri.parse(loc.uri),
              range: new monaco.Range(
                loc.line + 1, loc.character + 1,
                loc.end_line + 1, loc.end_character + 1
              ),
            }));
          } catch {
            return [];
          }
        },
      });
    }
  }, [editorInstance]);

  // Auto-start LSP when file is opened
  useEffect(() => {
    if (!editorInstance) return;

    const model = editorInstance.getModel();
    if (!model) return;

    const languageId = model.getLanguageId();
    const lspLang = getLanguageId(languageId);
    if (!lspLang) return;

    const projectPath = useAppStore.getState().projectPath;
    if (!projectPath) return;

    invoke('lsp_start', { projectPath, language: lspLang }).catch(() => {
      // LSP server not installed — silent failure
    });
  }, [editorInstance, useAppStore.getState().activeFile]);
}
