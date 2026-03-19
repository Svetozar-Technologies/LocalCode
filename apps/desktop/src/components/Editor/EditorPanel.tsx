import EditorTabs from './EditorTabs';
import MonacoEditor from './MonacoEditor';
import Breadcrumbs from './Breadcrumbs';
import { useAppStore } from '../../stores/appStore';

export default function EditorPanel() {
  const { activeFile, openFiles, projectPath } = useAppStore();
  const activeFileData = openFiles.find((f) => f.path === activeFile);

  return (
    <div className="editor-panel">
      <EditorTabs />
      {activeFileData && (
        <Breadcrumbs filePath={activeFileData.path} projectRoot={projectPath || undefined} />
      )}
      <div className="editor-content">
        <MonacoEditor />
      </div>
    </div>
  );
}
