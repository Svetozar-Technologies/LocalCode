import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import type { editor } from 'monaco-editor';

let completionDisposable: any = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let abortController: AbortController | null = null;

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
        token: any
      ) => {
        const { llmConnected } = useAppStore.getState();
        if (!llmConnected) return { items: [] };

        // Cancel any pending request
        if (abortController) {
          abortController.abort();
        }
        if (debounceTimer) {
          clearTimeout(debounceTimer);
        }

        // Check cancellation token
        if (token.isCancellationRequested) return { items: [] };

        // Debounce 300ms
        const result = await new Promise<{ items: any[] }>((resolve) => {
          debounceTimer = setTimeout(async () => {
            if (token.isCancellationRequested) {
              resolve({ items: [] });
              return;
            }

            abortController = new AbortController();
            useAppStore.getState().setCompletionStatus('completing');

            // Get file language for better context
            const uri = model.uri.toString();
            const lang = model.getLanguageId?.() || 'plaintext';

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

            // Prepend language hint to prompt
            const prompt = `// Language: ${lang}\n// File: ${uri.split('/').pop() || ''}\n${textBeforeCursor}`;

            try {
              const completion = await invoke<string>('llm_complete', {
                prompt,
                suffix: textAfterCursor,
              });

              useAppStore.getState().setCompletionStatus('idle');

              if (!completion || completion.trim().length === 0) {
                resolve({ items: [] });
                return;
              }

              resolve({
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
              });
            } catch {
              useAppStore.getState().setCompletionStatus('idle');
              resolve({ items: [] });
            }
          }, 300);
        });

        return result;
      },
      freeInlineCompletions: () => {},
    });

    return () => {
      if (completionDisposable) {
        completionDisposable.dispose();
        completionDisposable = null;
      }
      if (debounceTimer) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
      }
      if (abortController) {
        abortController.abort();
        abortController = null;
      }
    };
  }, [editorInstance]);
}
