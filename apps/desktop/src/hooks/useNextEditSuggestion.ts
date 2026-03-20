import { useEffect, useRef } from 'react';
import type { editor, IDisposable } from 'monaco-editor';
import type { Monaco } from '@monaco-editor/react';
import { invoke } from '@tauri-apps/api/core';

interface SuggestionState {
  decorations: string[];
  suggestedLine: number;
  suggestedText: string;
  disposable: IDisposable | null;
}

export function useNextEditSuggestion(editorInstance: editor.IStandaloneCodeEditor | null) {
  const stateRef = useRef<SuggestionState>({
    decorations: [],
    suggestedLine: -1,
    suggestedText: '',
    disposable: null,
  });
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!editorInstance) return;

    const monaco = (window as unknown as { monaco?: Monaco }).monaco;
    if (!monaco) return;

    const disposable = editorInstance.onDidChangeModelContent((event) => {
      // Clear previous suggestion
      const state = stateRef.current;
      state.decorations = editorInstance.deltaDecorations(state.decorations, []);
      state.suggestedLine = -1;

      // Debounce: wait 500ms after last edit
      if (debounceRef.current) clearTimeout(debounceRef.current);

      debounceRef.current = setTimeout(async () => {
        const model = editorInstance.getModel();
        if (!model) return;

        // Get the edit range context
        const changes = event.changes;
        if (changes.length === 0) return;

        const editLine = changes[0].range.startLineNumber;
        const totalLines = model.getLineCount();

        // Get surrounding context (10 lines before/after edit)
        const startLine = Math.max(1, editLine - 10);
        const endLine = Math.min(totalLines, editLine + 10);
        const contextLines: string[] = [];
        for (let i = startLine; i <= endLine; i++) {
          contextLines.push(model.getLineContent(i));
        }
        const editedText = changes[0].text || '';

        const prompt = `Given this code context around line ${editLine}:
\`\`\`
${contextLines.join('\n')}
\`\`\`

The user just edited line ${editLine}, changing it to include: "${editedText}"

Predict the next edit needed elsewhere in this file. Respond in EXACTLY this format:
LINE: <line_number>
TEXT: <replacement_text>

If no edit is needed, respond with: NONE`;

        try {
          const result = await invoke<string>('llm_complete', {
            prompt,
            suffix: '',
          });

          if (!result || result.includes('NONE')) return;

          // Parse the response
          const lineMatch = result.match(/LINE:\s*(\d+)/);
          const textMatch = result.match(/TEXT:\s*(.+)/);
          if (!lineMatch || !textMatch) return;

          const targetLine = parseInt(lineMatch[1], 10);
          const suggestionText = textMatch[1].trim();

          if (targetLine < 1 || targetLine > totalLines) return;

          // Show ghost text decoration at target line
          const state = stateRef.current;
          state.suggestedLine = targetLine;
          state.suggestedText = suggestionText;

          state.decorations = editorInstance.deltaDecorations(state.decorations, [
            {
              range: new monaco.Range(targetLine, 1, targetLine, 1),
              options: {
                isWholeLine: true,
                after: {
                  content: `  → ${suggestionText}`,
                  inlineClassName: 'next-edit-suggestion',
                },
                className: 'next-edit-suggestion-line',
              },
            },
          ]);
        } catch {
          // LLM not available — silently skip
        }
      }, 500);
    });

    stateRef.current.disposable = disposable;

    // Tab key to accept suggestion
    editorInstance.addCommand(
      monaco.KeyCode.Tab,
      () => {
        const state = stateRef.current;
        if (state.suggestedLine > 0 && state.suggestedText) {
          const model = editorInstance.getModel();
          if (!model) return;

          const pos = editorInstance.getPosition();
          if (!pos || pos.lineNumber !== state.suggestedLine) {
            // Default tab behavior if not on suggestion line
            editorInstance.trigger('keyboard', 'tab', null);
            return;
          }

          // Apply the suggestion
          const lineContent = model.getLineContent(state.suggestedLine);
          model.pushEditOperations([], [{
            range: new monaco.Range(state.suggestedLine, 1, state.suggestedLine, lineContent.length + 1),
            text: state.suggestedText,
          }], () => null);

          // Clear decoration
          state.decorations = editorInstance.deltaDecorations(state.decorations, []);
          state.suggestedLine = -1;
          state.suggestedText = '';
        } else {
          editorInstance.trigger('keyboard', 'tab', null);
        }
      },
    );

    return () => {
      disposable.dispose();
      if (debounceRef.current) clearTimeout(debounceRef.current);
      const state = stateRef.current;
      if (state.decorations.length > 0) {
        editorInstance.deltaDecorations(state.decorations, []);
      }
    };
  }, [editorInstance]);
}
