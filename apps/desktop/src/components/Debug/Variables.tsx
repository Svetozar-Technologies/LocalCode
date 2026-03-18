import { useState } from 'react';

interface VariableItem {
  name: string;
  value: string;
  type?: string;
  variablesReference: number;
}

interface VariablesProps {
  variables: VariableItem[];
}

const styles = {
  container: {
    fontSize: 12,
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
  } as React.CSSProperties,
  row: {
    display: 'flex',
    alignItems: 'center',
    padding: '2px 0',
    gap: 8,
    cursor: 'default',
  } as React.CSSProperties,
  name: {
    color: '#9cdcfe',
    flexShrink: 0,
  } as React.CSSProperties,
  separator: {
    color: '#6a6a6a',
  } as React.CSSProperties,
  value: {
    color: '#ce9178',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap' as const,
  } as React.CSSProperties,
  type: {
    color: '#4ec9b0',
    fontSize: 10,
    marginLeft: 'auto',
    flexShrink: 0,
  } as React.CSSProperties,
  empty: {
    color: '#6a6a6a',
    fontSize: 12,
    padding: '8px 0',
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  } as React.CSSProperties,
};

export default function Variables({ variables }: VariablesProps) {
  if (variables.length === 0) {
    return <div style={styles.empty}>No variables to display</div>;
  }

  return (
    <div style={styles.container}>
      {variables.map((v, i) => (
        <div
          key={`${v.name}-${i}`}
          style={styles.row}
          title={`${v.name}: ${v.value}${v.type ? ` (${v.type})` : ''}`}
        >
          <span style={styles.name}>{v.name}</span>
          <span style={styles.separator}>=</span>
          <span
            style={{
              ...styles.value,
              color: getValueColor(v.value, v.type),
            }}
          >
            {v.value}
          </span>
          {v.type && <span style={styles.type}>{v.type}</span>}
        </div>
      ))}
    </div>
  );
}

function getValueColor(value: string, type?: string): string {
  if (type === 'str' || type === 'string' || value.startsWith("'") || value.startsWith('"')) {
    return '#ce9178'; // String — orange
  }
  if (type === 'int' || type === 'float' || type === 'number' || /^\d/.test(value)) {
    return '#b5cea8'; // Number — green
  }
  if (value === 'True' || value === 'False' || value === 'true' || value === 'false') {
    return '#569cd6'; // Boolean — blue
  }
  if (value === 'None' || value === 'null' || value === 'undefined') {
    return '#569cd6'; // Null — blue
  }
  return '#cccccc'; // Default
}
