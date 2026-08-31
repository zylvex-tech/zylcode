import { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { StreamDelta } from "./events";

export interface ArtifactFile {
  path: string;
  kind: string;
  language: string;
  content: string;
  fullContent: string;
}

const ARTIFACT_TAG_OPEN = "<artifact";
const ARTIFACT_TAG_CLOSE = "</artifact>";
const CDATA_OPEN = "<![CDATA[";
const CDATA_CLOSE = "]]>";
const CONTENT_TAG_OPEN = "<content";
const CONTENT_TAG_CLOSE = "</content>";
const FENCE_OPEN = "```";

function detectLanguage(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "rs": return "rust";
    case "ts": return "typescript";
    case "tsx": return "typescript";
    case "js": return "javascript";
    case "jsx": return "javascript";
    case "json": return "json";
    case "yaml":
    case "yml": return "yaml";
    case "toml": return "toml";
    case "md": return "markdown";
    case "py": return "python";
    case "go": return "go";
    case "c": return "c";
    case "h": return "c";
    case "cpp": return "cpp";
    case "hpp": return "cpp";
    case "zig": return "zig";
    default: return "plaintext";
  }
}

function extractAttribute(tag: string, name: string): string | null {
  const needle = `${name}="`;
  const start = tag.indexOf(needle);
  if (start === -1) return null;
  const valueStart = start + needle.length;
  const end = tag.indexOf('"', valueStart);
  if (end === -1) return null;
  return tag.slice(valueStart, end);
}

function extractCdata(content: string): string {
  const start = content.indexOf(CDATA_OPEN);
  if (start === -1) return content;
  const end = content.indexOf(CDATA_CLOSE, start + CDATA_OPEN.length);
  if (end === -1) return content.slice(start + CDATA_OPEN.length);
  return content.slice(start + CDATA_OPEN.length, end);
}

function parseArtifactBlock(block: string): ArtifactFile | null {
  const tagEnd = block.indexOf(">");
  if (tagEnd === -1) return null;
  const tag = block.slice(0, tagEnd + 1);
  const kind = extractAttribute(tag, "kind") ?? "unknown";
  const path = extractAttribute(tag, "path") ?? `artifact.${kind}`;
  const language = extractAttribute(tag, "language") ?? detectLanguage(path);

  // Find inner content between > and </artifact>
  const innerStart = tagEnd + 1;
  const closeTagPos = block.indexOf(ARTIFACT_TAG_CLOSE, innerStart);
  if (closeTagPos === -1) return null;
  const inner = block.slice(innerStart, closeTagPos);

  // Check for <content> wrapper
  let content = inner;
  const contentStart = inner.indexOf(CONTENT_TAG_OPEN);
  if (contentStart !== -1) {
    const contentTagEnd = inner.indexOf(">", contentStart);
    if (contentTagEnd !== -1) {
      const cEnd = inner.indexOf(CONTENT_TAG_CLOSE, contentTagEnd + 1);
      if (cEnd !== -1) {
        content = inner.slice(contentTagEnd + 1, cEnd);
      }
    }
  }

  const fullContent = extractCdata(content).trim();
  return {
    path,
    kind,
    language,
    content: fullContent.slice(0, 10000),
    fullContent,
  };
}

function parseFenceBlock(block: string): ArtifactFile | null {
  const lines = block.split("\n");
  if (lines.length < 2) return null;
  const langLine = lines[0].trim();
  const lang = langLine.slice(3).trim() || "plaintext";
  const code = lines.slice(1).join("\n");
  const path = `snippet.${lang}`;
  return {
    path,
    kind: "Fence",
    language: lang,
    content: code.trim(),
    fullContent: code.trim(),
  };
}

export function useArtifactStream(deltas: StreamDelta[]) {
  const [files, setFiles] = useState<Map<string, ArtifactFile>>(new Map());
  const [activeTab, setActiveTab] = useState<string | null>(null);
  const [diffMode, setDiffMode] = useState(false);
  const [saveStatus, setSaveStatus] = useState<Record<string, "idle" | "saving" | "saved" | "error">>({});

  const bufferRef = useRef<string>("");
  const parsedRef = useRef<Map<string, ArtifactFile>>(new Map());

  // Process incoming deltas into artifact files
  useEffect(() => {
    if (!deltas.length) return;

    const newDeltas = deltas.slice(-50); // only process recent
    for (const delta of newDeltas) {
      if (!delta.delta) continue;
      bufferRef.current += delta.delta;

      // Process complete artifact blocks
      let searchPos = 0;
      while (true) {
        const openIdx = bufferRef.current.indexOf(ARTIFACT_TAG_OPEN, searchPos);
        if (openIdx === -1) break;
        const closeIdx = bufferRef.current.indexOf(ARTIFACT_TAG_CLOSE, openIdx);
        if (closeIdx === -1) break;
        const blockEnd = closeIdx + ARTIFACT_TAG_CLOSE.length;
        const block = bufferRef.current.slice(openIdx, blockEnd);
        const artifact = parseArtifactBlock(block);
        if (artifact) {
          parsedRef.current.set(artifact.path, artifact);
        }
        searchPos = blockEnd;
      }

      // Process fence blocks
      searchPos = 0;
      while (true) {
        const openIdx = bufferRef.current.indexOf(FENCE_OPEN, searchPos);
        if (openIdx === -1) break;
        const closeIdx = bufferRef.current.indexOf(FENCE_OPEN, openIdx + 3);
        if (closeIdx === -1) break;
        const block = bufferRef.current.slice(openIdx, closeIdx);
        const artifact = parseFenceBlock(block);
        if (artifact) {
          parsedRef.current.set(artifact.path, artifact);
        }
        searchPos = closeIdx;
      }
    }

    // Update state
    setFiles(new Map(parsedRef.current));
    if (activeTab === null && parsedRef.current.size > 0) {
      const firstKey = parsedRef.current.keys().next().value;
      if (firstKey) setActiveTab(firstKey);
    }
  }, [deltas, activeTab]);

  const saveFile = useCallback(async (path: string) => {
    const file = parsedRef.current.get(path);
    if (!file) return;
    setSaveStatus((s) => ({ ...s, [path]: "saving" }));
    try {
      await invoke("save_workspace_artifact", {
        label: path,
        content: file.fullContent,
      });
      setSaveStatus((s) => ({ ...s, [path]: "saved" }));
      setTimeout(() => setSaveStatus((s) => ({ ...s, [path]: "idle" })), 1500);
    } catch (e) {
      setSaveStatus((s) => ({ ...s, [path]: "error" }));
      console.error("Save failed:", e);
    }
  }, []);

  const applyPatch = useCallback(async (path: string) => {
    const file = parsedRef.current.get(path);
    if (!file) return;
    setSaveStatus((s) => ({ ...s, [path]: "saving" }));
    try {
      await invoke("apply_patch", {
        patch: file.fullContent,
      });
      setSaveStatus((s) => ({ ...s, [path]: "saved" }));
      setTimeout(() => setSaveStatus((s) => ({ ...s, [path]: "idle" })), 1500);
    } catch (e) {
      setSaveStatus((s) => ({ ...s, [path]: "error" }));
      console.error("Apply patch failed:", e);
    }
  }, []);

  const closeTab = useCallback((path: string) => {
    parsedRef.current.delete(path);
    setFiles(new Map(parsedRef.current));
    if (activeTab === path) {
      const remaining = parsedRef.current.keys().next().value;
      setActiveTab(remaining ?? null);
    }
  }, [activeTab]);

  return {
    files,
    activeTab,
    setActiveTab,
    diffMode,
    setDiffMode,
    saveStatus,
    saveFile,
    applyPatch,
    closeTab,
  };
}