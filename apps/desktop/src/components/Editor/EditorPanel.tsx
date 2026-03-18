import EditorTabs from './EditorTabs';
import MonacoEditor from './MonacoEditor';

export default function EditorPanel() {
  return (
    <div className="editor-panel">
      <EditorTabs />
      <div className="editor-content">
        <MonacoEditor />
      </div>
    </div>
  );
}
