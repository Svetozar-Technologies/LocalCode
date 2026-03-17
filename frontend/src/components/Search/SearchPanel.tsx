import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';
import type { SearchResult } from '../../types';

export default function SearchPanel() {
  const { projectPath, openFile } = useAppStore();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);

  const handleSearch = useCallback(async () => {
    if (!query.trim() || !projectPath) return;
    setSearching(true);
    try {
      const res = await invoke<SearchResult[]>('search_content', {
        path: projectPath,
        pattern: query,
      });
      setResults(res);
    } catch (err) {
      console.error('Search failed:', err);
      setResults([]);
    }
    setSearching(false);
  }, [query, projectPath]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSearch();
  };

  // Group results by file
  const grouped: Record<string, SearchResult[]> = {};
  results.forEach((r) => {
    if (!grouped[r.file]) grouped[r.file] = [];
    grouped[r.file].push(r);
  });

  const handleResultClick = async (result: SearchResult) => {
    try {
      const content = await invoke<string>('read_file', { path: result.file });
      const ext = result.file.split('.').pop() || '';
      openFile({
        path: result.file,
        name: result.file.split('/').pop() || '',
        content,
        language: ext,
        modified: false,
      });
    } catch (err) {
      console.error('Failed to open file:', err);
    }
  };

  return (
    <div className="search-panel">
      <input
        className="search-input"
        type="text"
        placeholder="Search in files..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={handleKeyDown}
      />
      {!projectPath && (
        <p style={{ color: 'var(--text-muted)', fontSize: 11, marginTop: 8 }}>
          Open a folder to search
        </p>
      )}
      <div className="search-results">
        {searching && <p style={{ color: 'var(--text-muted)', fontSize: 11 }}>Searching...</p>}
        {Object.entries(grouped).map(([file, matches]) => (
          <div key={file}>
            <div className="search-result-file">
              {file.replace(projectPath + '/', '')} ({matches.length})
            </div>
            {matches.map((match, i) => (
              <div
                key={i}
                className="search-result-line"
                onClick={() => handleResultClick(match)}
              >
                <span style={{ color: 'var(--text-muted)', marginRight: 8 }}>{match.line}</span>
                {match.content}
              </div>
            ))}
          </div>
        ))}
        {!searching && results.length === 0 && query && (
          <p style={{ color: 'var(--text-muted)', fontSize: 11, marginTop: 8 }}>No results</p>
        )}
      </div>
    </div>
  );
}
