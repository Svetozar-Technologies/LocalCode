import { useAppStore } from '../../stores/appStore';

export default function EditorTabs() {
  const { openFiles, activeFile, setActiveFile, closeFile } = useAppStore();

  if (openFiles.length === 0) return null;

  return (
    <div className="editor-tabs">
      {openFiles.map((file) => (
        <div
          key={file.path}
          className={`editor-tab ${activeFile === file.path ? 'active' : ''}`}
          onClick={() => setActiveFile(file.path)}
        >
          {file.modified && <span className="tab-modified" />}
          <span className="tab-name">{file.name}</span>
          <span
            className="tab-close"
            onClick={(e) => {
              e.stopPropagation();
              closeFile(file.path);
            }}
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 8.707l3.646 3.647.708-.707L8.707 8l3.647-3.646-.707-.708L8 7.293 4.354 3.646l-.707.708L7.293 8l-3.646 3.646.707.708L8 8.707z" />
            </svg>
          </span>
        </div>
      ))}
    </div>
  );
}
