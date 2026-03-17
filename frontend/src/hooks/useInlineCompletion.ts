import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import type { editor } from 'monaco-editor';

let completionDisposable: any = null;

export function useInlineCompletion(editorInstance: editor.IStandaloneCodeEditor | null) {

  useEffect(() => {
    if (!editorInstance) return;

    const monaco = (window as any).monaco;
    if (!monaco) return;

    // Dispose previous provider
    if (completionDisposable) {
      completionDisposable.dispose();
      completionDisposable = null;
    }

    // Register inline completion provider
    completionDisposable = monaco.languages.registerInlineCompletionsProvider('*', {
      provideInlineCompletions: async (
        model: editor.ITextModel,
        position: any,
        _context: any,
        _token: any
      ) => {
        const { llmConnected } = useAppStore.getState();
        if (!llmConnected) return { items: [] };

        // Get text before and after cursor
        const textBeforeCursor = model.getValueInRange({
          startLineNumber: Math.max(1, position.lineNumber - 50),
          startColumn: 1,
          endLineNumber: position.lineNumber,
          endColumn: position.column,
        });

        const textAfterCursor = model.getValueInRange({
          startLineNumber: position.lineNumber,
          startColumn: position.column,
          endLineNumber: Math.min(model.getLineCount(), position.lineNumber + 10),
          endColumn: model.getLineMaxColumn(
            Math.min(model.getLineCount(), position.lineNumber + 10)
          ),
        });

        try {
          const completion = await invoke<string>('llm_complete', {
            prompt: textBeforeCursor,
            suffix: textAfterCursor,
          });

          if (!completion || completion.trim().length === 0) {
            return { items: [] };
          }

          return {
            items: [
              {
                insertText: completion,
                range: {
                  startLineNumber: position.lineNumber,
                  startColumn: position.column,
                  endLineNumber: position.lineNumber,
                  endColumn: position.column,
                },
              },
            ],
          };
        } catch {
          return { items: [] };
        }
      },
      freeInlineCompletions: () => {},
    });

    return () => {
      if (completionDisposable) {
        completionDisposable.dispose();
        completionDisposable = null;
      }
    };
  }, [editorInstance]);
}
