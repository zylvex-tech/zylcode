import React, { useEffect, useRef } from "react";
import { Editor, DiffEditor } from "@monaco-editor/react";
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

const MONACO_THEME = "vs-dark";

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
  const [originalContent, setOriginalContent] = React.useState<string>("");

  // Load original workspace content when diff mode is enabled
  useEffect(() => {
    if (!diffMode || !activeTab) return;
    // In a real implementation, this would read the original file from disk
    // For now, we'll use empty string to show all content as additions
    setOriginalContent("");
  }, [diffMode, activeTab]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
      const modifier = isMac ? e.metaKey : e.ctrlKey;

      if (modifier && e.key === "s") {
        e.preventDefault();
        if (editingFile) {
          handleSave();
        }
      }
      if (modifier && e.key === "Enter") {
        e.preventDefault();
        if (activeTab && !editingFile) {
          handleApplyPatch();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [editingFile, activeTab, saveFile, applyPatch]);

  const handleSave = async () => {
    if (!editingFile) return;
    await saveFile(editingFile);
    setSaveBanner({ show: true, message: "File saved!", type: "success" });
    setTimeout(() => setSaveBanner({ show: false, message: "", type: "success" }), 2000);
  };

  const handleApplyPatch = async () => {
    if (!activeTab) return;
    await applyPatch(activeTab);
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

  // Get language for current file
  const getLanguage = (path: string) => {
    const ext = path.split(".").pop()?.toLowerCase() ?? "";
    return EXT_LANGUAGE_MAP[ext] || "plaintext";
  };

  // Monaco editor options
  const editorOptions = React.useMemo(() => ({
    theme: MONACO_THEME,
    automaticLayout: true,
    fontSize: 13,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    renderWhitespace: "selection" as const,
  }), []);

  // Diff editor options
  const diffEditorOptions = React.useMemo(() => ({
    theme: MONACO_THEME,
    renderSideBySide: true,
    enableSplitViewResizing: true,
    automaticLayout: true,
    fontSize: 13,
    minimap: { enabled: false },
    readOnly: true,
  }), []);

  let fileDisplay: React.ReactNode;

  if (isEditing && editingFile && activeFile) {
    fileDisplay = (
      <div className="flex-1 min-h-0">
        <Editor
          height="100%"
          language={getLanguage(editingFile)}
          value={fileContent || activeFile.content}
          options={editorOptions}
          onChange={(value) => value && setFileContent(value)}
          theme={MONACO_THEME}
        />
      </div>
    );
  } else if (activeFile && diffMode) {
    // Diff view - use Monaco DiffEditor
    const language = getLanguage(activeTab!);
    fileDisplay = (
      <div className="flex-1 min-h-0" style={{ height: "100%" }}>
        <DiffEditor
          height="100%"
          language={language}
          original={originalContent}
          modified={activeFile.content}
          options={diffEditorOptions}
          theme={MONACO_THEME}
        />
      </div>
    );
  } else if (activeFile) {
    // Code view - use Monaco Editor (read-only)
    fileDisplay = (
      <div className="flex-1 min-h-0">
        <Editor
          height="100%"
          language={getLanguage(activeTab!)}
          value={activeFile.content}
          options={{ ...editorOptions, readOnly: true }}
          theme={MONACO_THEME}
        />
      </div>
    );
  } else {
    fileDisplay = (
      <p className="p-4 text-zyl-muted text-center">Select a file tab to preview</p>
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
          <div className="text-[10px] text-zyl-muted px-2 pt-1 font-mono">
            {isMac() ? "⌘S save · ⌘⏎ apply" : "Ctrl+S save · Ctrl+⏎ apply"}
          </div>
        </div>
      </header>

      <div className="flex flex-col space-y-2">
        {/* Tab Bar */}
        <div className="flex gap-1 rounded-md overflow-hidden bg-zyl-surface">
          {tabNodes}
        </div>

        {/* Active File Area */}
        <div className="flex-1 min-h-0">
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

function isMac() {
  return typeof navigator !== "undefined" && navigator.platform.toUpperCase().indexOf("MAC") >= 0;
}

export default ArtifactViewer;