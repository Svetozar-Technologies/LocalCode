import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';

interface MapNode {
  id: string;
  label: string;
  deps: string[];
  x: number;
  y: number;
}

const IMPORT_REGEX = /(?:import\s+.*?from\s+['"](.+?)['"]|require\(['"](.+?)['"]\))/g;

function extractImports(content: string): string[] {
  const imports: string[] = [];
  let match: RegExpExecArray | null;
  const regex = new RegExp(IMPORT_REGEX.source, 'g');
  while ((match = regex.exec(content)) !== null) {
    const dep = match[1] || match[2];
    if (dep && !dep.startsWith('.')) continue; // Only local imports
    if (dep) imports.push(dep);
  }
  return imports;
}

function resolveRelativePath(fromFile: string, importPath: string): string {
  const fromDir = fromFile.substring(0, fromFile.lastIndexOf('/'));
  const parts = `${fromDir}/${importPath}`.split('/');
  const resolved: string[] = [];
  for (const part of parts) {
    if (part === '..') resolved.pop();
    else if (part !== '.') resolved.push(part);
  }
  return resolved.join('/');
}

export default function CodebaseMap() {
  const { projectPath, toggleCodebaseMap } = useAppStore();
  const [nodes, setNodes] = useState<MapNode[]>([]);
  const [loading, setLoading] = useState(true);

  const buildMap = useCallback(async () => {
    if (!projectPath) return;
    setLoading(true);

    try {
      const files = await invoke<{ path: string; name: string }[]>('search_files', {
        path: projectPath,
        query: '',
      });

      // Filter to code files
      const codeFiles = files.filter((f) => {
        const ext = f.name.split('.').pop()?.toLowerCase() || '';
        return ['ts', 'tsx', 'js', 'jsx', 'py', 'rs', 'go'].includes(ext);
      }).slice(0, 100); // Limit to 100 files

      const nodeMap: Record<string, MapNode> = {};

      // Create nodes
      for (const file of codeFiles) {
        const relPath = file.path.replace(projectPath + '/', '');
        nodeMap[relPath] = {
          id: relPath,
          label: file.name,
          deps: [],
          x: 0,
          y: 0,
        };
      }

      // Parse dependencies
      for (const file of codeFiles) {
        try {
          const content = await invoke<string>('read_file', { path: file.path });
          const imports = extractImports(content);
          const relPath = file.path.replace(projectPath + '/', '');

          for (const imp of imports) {
            const resolved = resolveRelativePath(relPath, imp);
            // Try to find matching file (with extensions)
            const candidates = [resolved, `${resolved}.ts`, `${resolved}.tsx`, `${resolved}.js`, `${resolved}/index.ts`, `${resolved}/index.tsx`];
            for (const candidate of candidates) {
              if (nodeMap[candidate]) {
                nodeMap[relPath].deps.push(candidate);
                break;
              }
            }
          }
        } catch {
          // Skip unreadable files
        }
      }

      // Simple layout: arrange in a grid with connected nodes nearby
      const allNodes = Object.values(nodeMap);
      const cols = Math.ceil(Math.sqrt(allNodes.length));
      allNodes.forEach((node, i) => {
        node.x = (i % cols) * 160 + 20;
        node.y = Math.floor(i / cols) * 60 + 20;
      });

      setNodes(allNodes);
    } catch {
      setNodes([]);
    }

    setLoading(false);
  }, [projectPath]);

  useEffect(() => {
    if (!projectPath) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    buildMap();
  }, [projectPath, buildMap]);

  if (loading) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-muted)' }}>
        Building codebase map...
      </div>
    );
  }

  const nodeById = Object.fromEntries(nodes.map((n) => [n.id, n]));

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{
        display: 'flex',
        alignItems: 'center',
        padding: '6px 12px',
        borderBottom: '1px solid var(--border-color)',
        fontSize: 12,
        color: 'var(--text-primary)',
        gap: 8,
      }}>
        <span style={{ fontWeight: 600 }}>Codebase Map</span>
        <span style={{ color: 'var(--text-muted)', fontSize: 11 }}>{nodes.length} files</span>
        <button
          onClick={toggleCodebaseMap}
          style={{
            marginLeft: 'auto',
            background: 'none',
            border: 'none',
            color: 'var(--text-muted)',
            cursor: 'pointer',
          }}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 8.707l3.646 3.647.708-.707L8.707 8l3.647-3.646-.707-.708L8 7.293 4.354 3.646l-.707.708L7.293 8l-3.646 3.646.707.708L8 8.707z" />
          </svg>
        </button>
      </div>
      <div className="codebase-map" style={{ position: 'relative', overflow: 'auto', flex: 1 }}>
        {/* Draw edges as SVG lines */}
        <svg style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', pointerEvents: 'none' }}>
          {nodes.map((node) =>
            node.deps.map((dep) => {
              const target = nodeById[dep];
              if (!target) return null;
              return (
                <line
                  key={`${node.id}-${dep}`}
                  x1={node.x + 70}
                  y1={node.y + 14}
                  x2={target.x + 70}
                  y2={target.y + 14}
                  stroke="var(--border-color)"
                  strokeWidth={1}
                  opacity={0.5}
                />
              );
            })
          )}
        </svg>
        {/* Draw nodes */}
        {nodes.map((node) => (
          <div
            key={node.id}
            className="codebase-map-node"
            style={{ left: node.x, top: node.y }}
            title={node.id}
            onClick={() => {
              const store = useAppStore.getState();
              if (store.projectPath) {
                invoke<string>('read_file', { path: `${store.projectPath}/${node.id}` })
                  .then((content) => {
                    store.openFile({
                      path: `${store.projectPath}/${node.id}`,
                      name: node.label,
                      content,
                      language: 'typescript',
                      modified: false,
                    });
                    store.toggleCodebaseMap();
                  })
                  .catch(() => {});
              }
            }}
          >
            {node.label}
            {node.deps.length > 0 && (
              <span style={{ color: 'var(--text-muted)', fontSize: 9, marginLeft: 4 }}>
                ({node.deps.length})
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
