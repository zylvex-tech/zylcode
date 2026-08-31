import React, { useEffect } from "react";
import { useArtifactStream } from "../lib/useArtifactStream";
import { ArtifactFile } from "../lib/useArtifactStream";

export interface ArtifactViewerProps {
  files: Map<string, ArtifactFile>;
  activeTab: string | null;
  setActiveTab: React.Dispatch<React.SetStateAction<string | null>>;
  diffMode: boolean;
  setDiffMode: React.Dispatch<React.SetStateAction<boolean>>;
  saveStatus: Record<string, "idle" | "saving" | "saved" | "error">;
  saveFile: (path: string) => Promise<void>;
  applyPatch: (path: string) => Promise<void>;
  closeTab: (path: string) => void;
}

const EXT_LANGUAGE_MAP: Record<string, string> = {
  rs: "rust",
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  json: "json",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  md: "markdown",
  py: "python",
  go: "go",
  c: "c",
  h: "c",
  cpp: "cpp",
  hpp: "cpp",
  zig: "zig",
};

export const ArtifactViewer: React.FC<ArtifactViewerProps> = ({
  files,
  activeTab,
  setActiveTab,
  diffMode,
  setDiffMode,
  saveStatus,
  saveFile,
  applyPatch,
  closeTab,
}) => {
  const [editingFile, setEditingFile] = React.useState<string | null>(null);
  const [fileContent, setFileContent] = React.useState<string>("");
  const [saveBanner, setSaveBanner] = React.useState<{
    show: boolean;
    message: string;
    type: "success" | "error";
  }>({ show: false, message: "", type: "success" });

  useEffect(() => {
    if (!editingFile) return;
    const file = files.get(editingFile!);
    if (!file) return;
    const ext = editingFile.split(".").pop()?.toLowerCase() ?? "";
    const lang = EXT_LANGUAGE_MAP[ext] || "plaintext";
    const container = document.getElementById(`monaco-editor-${editingFile.replace(/\./g, "-")}`);
    if (!container) return;
    if ((window as any).monaco) {
      // @ts-ignore
      (window as any).monaco.editor.create(container, {
        value: file.content,
        language: lang as any,
        automaticLayout: true,
        fontSize: 13,
        theme: "vs-dark",
        minimap: { enabled: false },
      });
    }
  }, [editingFile, files]);

  const handleSave = async () => {
    if (!editingFile) return;
    await saveFile(editingFile);
    setSaveBanner({ show: true, message: "File saved!", type: "success" });
    setTimeout(() => setSaveBanner({ show: false, message: "", type: "success" }), 2000);
  };

  const handleApplyPatch = async () => {
    if (!editingFile) return;
    await applyPatch(editingFile);
    setSaveBanner({ show: true, message: "Patch applied!", type: "success" });
    setTimeout(() => setSaveBanner({ show: false, message: "", type: "success" }), 2000);
  };

  const handleClose = (e: React.MouseEvent) => {
    e.stopPropagation();
    const path = activeTab!;
    closeTab(path);
    setActiveTab(path === activeTab ? null : activeTab);
  };

  if (files.size === 0) {
    return (
      <div className="p-8 text-zyl-muted text-center">
        <p>No artifacts yet. Run an intent to generate files.</p>
      </div>
    );
  }

  const activeFile = activeTab ? files.get(activeTab) ?? null : null;
  const isEditing = editingFile !== null;

  const tabNodes = Array.from(files.keys()).map((path) => {
    const file = files.get(path)!;
    const ext = path.split(".").pop()?.toLowerCase() ?? "";
    const lang = EXT_LANGUAGE_MAP[ext] || "plaintext";
    const isActive = activeTab === path;
    return (
      <div
        key={path}
        className={`flex items-center rounded-md px-2.5 py-1.5 text-sm font-medium transition-colors ${isActive ? "bg-zyl-accent text-white" : "text-zyl-muted hover:bg-zyl-surface"}`
      }
        onClick={() => setActiveTab(path)}
      >
        {path}
      </div>
    );
  });

  // Compute the file content display section
  let fileDisplay: React.ReactNode;
  if (isEditing && editingFile) {
    fileDisplay = (
      <div className="flex-1">
        <textarea
          value={fileContent}
          onChange={(e) => setFileContent(e.target.value)}
          placeholder="Start editing..."
          className="w-full h-full text-sm font-mono resize-none bg-zyl-bg text-white outline-none"
        />
      </div>
    );
  } else if (activeFile) {
    const showDiff = diffMode && !isEditing;
    if (showDiff && activeFile) {
      fileDisplay = (
        <div className="flex-1">
          <div className="mt-3 p-3 border-t border-zyl-border">
            <h3 className="text-[10px] font-semibold uppercase tracking-widest text-zyl-muted mb-2">Diff Preview</h3>
            <pre className="text-[10px] font-mono text-zyl-muted whitespace-pre-wrap break-words">
              {activeFile.content.slice(0, 2000)}
            </pre>
          </div>
        </div>
      );
    } else if (!showDiff && activeFile) {
      fileDisplay = (
        <div className="flex-1">
          <div className="mt-3 p-3 border-t border-zyl-border">
            <pre className="text-[10px] font-mono text-zyl-muted whitespace-pre-wrap break-words">
              {activeFile.content.slice(0, 2000)}
            </pre>
          </div>
        </div>
      );
    } else {
      fileDisplay = (
        <p className="p-4 text-zyl-muted text-center">Select a file tab to preview</p>
      );
    }
  } else {
    fileDisplay = (
      <p className="p-4 text-zyl-muted text-center">No active file</p>
    );
  }

  return (
    <div className="min-h-screen bg-zyl-bg text-white p-4">
      <header className="border-b border-zyl-border bg-zyl-surface mb-4 p-3 flex items-center justify-between">
        <h2 className="text-lg font-bold tracking-tight">Artifact Viewer</h2>
        <div className="flex gap-2">
          <button
            onClick={() => setDiffMode(!diffMode)}
            className={`
              rounded-md border border-zyl-border px-3 py-1.5 text-sm font-semibold transition-colors
              ${diffMode ? "bg-zyl-accent text-white" : "text-zyl-muted hover:bg-zyl-surface"}
            `}
            title="Toggle Diff View"
          >
            {diffMode ? "Code View" : "Diff View"}
          </button>
          <button
            onClick={() => setEditingFile(activeTab ?? null)}
            disabled={isEditing || !activeTab}
            className={`
              rounded-md border border-zyl-border px-3 py-1.5 text-sm font-semibold transition-colors
              ${isEditing ? "bg-zyl-accent text-white" : "text-zyl-muted hover:bg-zyl-surface"}
            `}
            title={isEditing ? "Exit Editing" : "Start Editing"}
          >
            {isEditing ? "Done" : "Edit"}
          </button>
        </div>
      </header>

      <div className="flex flex-col space-y-2">
        {/* Tab Bar */}
        <div className="flex gap-1 rounded-md overflow-hidden bg-zyl-surface">
          {tabNodes}
        </div>

        {/* Active File Area */}
        <div className="flex-1">
          {fileDisplay}
        </div>
      </div>

      {/* Save banner */}
      {saveBanner.show && (
        <div className="fixed bottom-4 left-1/2 -translate-x-1/2 rounded-md px-4 py-2 text-sm font-medium transition-colors">
          {saveBanner.type === "success" ? "bg-zyl-accent text-white" : "bg-red-600 text-white"}
          {saveBanner.message}
        </div>
      )}
    </div>
  );
};

export default ArtifactViewer;