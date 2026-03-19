import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../stores/appStore';
import type { editor } from 'monaco-editor';

let completionDisposable: any = null;
let cursorDisposable: any = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let abortController: AbortController | null = null;

// LRU cache: 10 entries, 60s TTL
const CACHE_MAX = 10;
const CACHE_TTL = 60_000;
interface CacheEntry {
  completion: string;
  timestamp: number;
}
const completionCache = new Map<string, CacheEntry>();

function cacheGet(prefix: string): string | null {
  const entry = completionCache.get(prefix);
  if (!entry) return null;
  if (Date.now() - entry.timestamp > CACHE_TTL) {
    completionCache.delete(prefix);
    return null;
  }
  // Move to end (most recently used)
  completionCache.delete(prefix);
  completionCache.set(prefix, entry);
  return entry.completion;
}

function cacheSet(prefix: string, completion: string) {
  // Evict oldest if at capacity
  if (completionCache.size >= CACHE_MAX) {
    const oldest = completionCache.keys().next().value;
    if (oldest !== undefined) completionCache.delete(oldest);
  }
  completionCache.set(prefix, { completion, timestamp: Date.now() });
}

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
    if (cursorDisposable) {
      cursorDisposable.dispose();
      cursorDisposable = null;
    }

    // Cancel on cursor movement
    cursorDisposable = editorInstance.onDidChangeCursorPosition(() => {
      if (debounceTimer) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
      }
      if (abortController) {
        abortController.abort();
        abortController = null;
      }
    });

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

        // Get text before cursor for cache key
        const textBeforeCursor = model.getValueInRange({
          startLineNumber: Math.max(1, position.lineNumber - 50),
          startColumn: 1,
          endLineNumber: position.lineNumber,
          endColumn: position.column,
        });

        // Check cache first
        const cached = cacheGet(textBeforeCursor);
        if (cached) {
          return {
            items: [
              {
                insertText: cached,
                range: {
                  startLineNumber: position.lineNumber,
                  startColumn: position.column,
                  endLineNumber: position.lineNumber,
                  endColumn: position.column,
                },
              },
            ],
          };
        }

        // Debounce 200ms
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

              // Cache successful completion
              cacheSet(textBeforeCursor, completion);

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
          }, 200);
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
      if (cursorDisposable) {
        cursorDisposable.dispose();
        cursorDisposable = null;
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
