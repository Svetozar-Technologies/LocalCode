import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from '../stores/appStore';
import type { editor } from 'monaco-editor';

let completionDisposable: any = null;
let cursorDisposable: any = null;
let fileChangeDisposable: any = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let abortController: AbortController | null = null;

// --- Improved Cache: 500 entries, 5min TTL, prefix-aware matching ---
const CACHE_MAX = 500;
const CACHE_TTL = 5 * 60_000; // 5 minutes

interface CacheEntry {
  completion: string;
  timestamp: number;
  key: string; // original full key for prefix matching
}

const completionCache = new Map<string, CacheEntry>();

function hashCacheKey(lastLines: string, cursorLine: number, cursorCol: number): string {
  // Use last 5 lines + cursor position as the cache key (fast lookups)
  const lines = lastLines.split('\n');
  const last5 = lines.slice(-5).join('\n');
  return `${last5}|${cursorLine}:${cursorCol}`;
}

function cacheGet(key: string): string | null {
  // Exact match first
  const entry = completionCache.get(key);
  if (entry) {
    if (Date.now() - entry.timestamp > CACHE_TTL) {
      completionCache.delete(key);
    } else {
      // Move to end (most recently used)
      completionCache.delete(key);
      completionCache.set(key, entry);
      return entry.completion;
    }
  }

  // Prefix-aware matching: check if any cached key is a prefix of the current key
  for (const [cachedKey, cachedEntry] of completionCache) {
    if (Date.now() - cachedEntry.timestamp > CACHE_TTL) {
      completionCache.delete(cachedKey);
      continue;
    }
    // If the current key starts with a cached key (user typed more)
    if (key.startsWith(cachedKey) && key.length > cachedKey.length) {
      const extraChars = key.length - cachedKey.length;
      const remaining = cachedEntry.completion.slice(extraChars);
      if (remaining.length > 0) {
        return remaining;
      }
    }
  }

  return null;
}

function cacheSet(key: string, completion: string) {
  // Evict oldest if at capacity
  if (completionCache.size >= CACHE_MAX) {
    const oldest = completionCache.keys().next().value;
    if (oldest !== undefined) completionCache.delete(oldest);
  }
  completionCache.set(key, { completion, timestamp: Date.now(), key });
}

function cacheClear() {
  completionCache.clear();
}

// --- Per-language stop tokens ---
function getLanguageStops(lang: string): string[] {
  switch (lang) {
    case 'python':
      return ['\ndef ', '\nclass ', '\n# '];
    case 'rust':
      return ['\nfn ', '\nimpl ', '\nstruct ', '\nenum ', '\npub fn ', '\npub struct '];
    case 'javascript':
    case 'typescript':
    case 'javascriptreact':
    case 'typescriptreact':
      return ['\nfunction ', '\nclass ', '\nexport ', '\nconst ', '\nlet '];
    case 'go':
      return ['\nfunc ', '\ntype '];
    case 'java':
    case 'kotlin':
      return ['\npublic ', '\nprivate ', '\nprotected ', '\nclass '];
    case 'c':
    case 'cpp':
      return ['\nint ', '\nvoid ', '\nstruct ', '\nclass ', '\n#include'];
    default:
      return [];
  }
}

// --- Detect multiline context ---
function shouldRequestMultiline(textBeforeCursor: string): boolean {
  const trimmed = textBeforeCursor.trimEnd();
  if (trimmed.length === 0) return false;
  const lastChar = trimmed[trimmed.length - 1];
  // After opening brace, colon (Python), arrow (=>), or paren for function def
  if ('{:('.includes(lastChar)) return true;
  if (trimmed.endsWith('=>')) return true;
  // After function/class/if/for/while signatures
  const lastLine = trimmed.split('\n').pop() || '';
  if (/^\s*(def |fn |func |function |class |if |for |while |match |switch )/.test(lastLine)) {
    return true;
  }
  return false;
}

// --- Find enclosing function/class signature ---
function findEnclosingSignature(fullText: string, cursorLine: number): string {
  const lines = fullText.split('\n');
  // Scan backward from cursor to find enclosing function/class definition
  for (let i = Math.min(cursorLine - 1, lines.length - 1); i >= 0; i--) {
    const line = lines[i];
    if (/^\s*(def |fn |func |function |class |impl |pub fn |pub struct |export (function|class|const))/.test(line)) {
      return line;
    }
  }
  return '';
}

// --- Extract imports block (first 30 lines) ---
function getImportsBlock(fullText: string): string {
  const lines = fullText.split('\n');
  return lines.slice(0, 30).join('\n');
}

export function useInlineCompletion(editorInstance: editor.IStandaloneCodeEditor | null) {
  const lastFileRef = useRef<string | null>(null);

  useEffect(() => {
    if (!editorInstance) return;

    const monaco = (window as any).monaco;
    if (!monaco) return;

    // Dispose previous providers
    if (completionDisposable) {
      completionDisposable.dispose();
      completionDisposable = null;
    }
    if (cursorDisposable) {
      cursorDisposable.dispose();
      cursorDisposable = null;
    }
    if (fileChangeDisposable) {
      fileChangeDisposable.dispose();
      fileChangeDisposable = null;
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

    // Clear cache on file switch
    fileChangeDisposable = editorInstance.onDidChangeModel(() => {
      const newUri = editorInstance.getModel()?.uri.toString() || null;
      if (newUri !== lastFileRef.current) {
        cacheClear();
        lastFileRef.current = newUri;
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
        const { llmConnected, selectedProvider } = useAppStore.getState();
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

        const fullText = model.getValue();
        const lineCount = model.getLineCount();
        const lang = model.getLanguageId?.() || 'plaintext';

        // Expanded context: 150 lines before, 30 lines after
        const textBeforeCursor = model.getValueInRange({
          startLineNumber: Math.max(1, position.lineNumber - 150),
          startColumn: 1,
          endLineNumber: position.lineNumber,
          endColumn: position.column,
        });

        // Cache key: hash of last 5 lines + cursor position
        const cacheKey = hashCacheKey(textBeforeCursor, position.lineNumber, position.column);

        // Check cache first (exact + prefix-aware)
        const cached = cacheGet(cacheKey);
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

            const uri = model.uri.toString();
            const filename = uri.split('/').pop() || '';

            const textAfterCursor = model.getValueInRange({
              startLineNumber: position.lineNumber,
              startColumn: position.column,
              endLineNumber: Math.min(lineCount, position.lineNumber + 30),
              endColumn: model.getLineMaxColumn(
                Math.min(lineCount, position.lineNumber + 30)
              ),
            });

            // Build enriched prompt
            const importsBlock = getImportsBlock(fullText);
            const enclosingSignature = findEnclosingSignature(fullText, position.lineNumber);

            let prompt = `// Language: ${lang}\n// File: ${filename}\n`;
            prompt += `// Imports:\n${importsBlock}\n// ...\n`;
            if (enclosingSignature) {
              prompt += `// Enclosing: ${enclosingSignature.trim()}\n`;
            }
            prompt += `// Context:\n${textBeforeCursor}`;

            // Detect multiline context
            const multiline = shouldRequestMultiline(textBeforeCursor);

            // Per-language stop tokens
            const langStops = getLanguageStops(lang);
            const stop = multiline
              ? langStops
              : ['\n\n', '\r\n\r\n', ...langStops];

            // Use streaming for local provider, non-streaming for cloud
            const useStreaming = selectedProvider === 'local';

            try {
              if (useStreaming) {
                // Streaming mode
                const responseId = `completion-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
                let completionText = '';
                let done = false;

                const unlistenChunk = await listen<{ id: string; chunk: string; error?: string }>(
                  'llm-completion-chunk',
                  (event) => {
                    if (event.payload.id === responseId) {
                      if (event.payload.error) {
                        done = true;
                      } else {
                        completionText += event.payload.chunk;
                      }
                    }
                  }
                );

                const unlistenDone = await listen<{ id: string }>(
                  'llm-completion-done',
                  (event) => {
                    if (event.payload.id === responseId) {
                      done = true;
                    }
                  }
                );

                // Fire the streaming request (don't await the full result)
                invoke('llm_complete_stream', {
                  responseId,
                  prompt,
                  suffix: textAfterCursor,
                  providerName: selectedProvider,
                  multiline,
                  stop,
                }).catch(() => {
                  done = true;
                });

                // Wait for completion with timeout
                const startTime = Date.now();
                while (!done && Date.now() - startTime < 8000) {
                  await new Promise((r) => setTimeout(r, 50));
                  if (token.isCancellationRequested) {
                    unlistenChunk();
                    unlistenDone();
                    useAppStore.getState().setCompletionStatus('idle');
                    resolve({ items: [] });
                    return;
                  }
                }

                unlistenChunk();
                unlistenDone();
                useAppStore.getState().setCompletionStatus('idle');

                if (!completionText || completionText.trim().length === 0) {
                  resolve({ items: [] });
                  return;
                }

                cacheSet(cacheKey, completionText);

                resolve({
                  items: [
                    {
                      insertText: completionText,
                      range: {
                        startLineNumber: position.lineNumber,
                        startColumn: position.column,
                        endLineNumber: position.lineNumber,
                        endColumn: position.column,
                      },
                    },
                  ],
                });
              } else {
                // Non-streaming mode (cloud providers)
                const completion = await invoke<string>('llm_complete', {
                  prompt,
                  suffix: textAfterCursor,
                  providerName: selectedProvider,
                  multiline,
                  stop,
                });

                useAppStore.getState().setCompletionStatus('idle');

                if (!completion || completion.trim().length === 0) {
                  resolve({ items: [] });
                  return;
                }

                cacheSet(cacheKey, completion);

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
              }
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
      if (fileChangeDisposable) {
        fileChangeDisposable.dispose();
        fileChangeDisposable = null;
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
