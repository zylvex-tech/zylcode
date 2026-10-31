import { useState } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ArtifactViewer, type ArtifactViewerProps } from "./ArtifactViewer";
import type { ArtifactFile } from "../lib/useArtifactStream";

// ---------------------------------------------------------------------------
// ArtifactViewer is the operator's window onto generated output, so the tests
// pin the three things that actually matter here:
//   * it never claims content it does not have (empty state / no tab);
//   * the keyboard contract — Ctrl+S saves the edited file, Ctrl+Enter applies
//     a patch, and the two are mutually exclusive by edit state;
//   * hooks are registered unconditionally (the file used to call a different
//     number of them depending on whether a tab was open).
// Monaco itself is stubbed: it is a canvas host, not the logic under test.
// ---------------------------------------------------------------------------

type MonacoProps = {
  value?: string;
  language?: string;
  options?: Record<string, unknown>;
  theme?: string;
  height?: string;
  onChange?: (value?: string) => void;
};

type DiffProps = {
  original?: string;
  modified?: string;
  language?: string;
  options?: Record<string, unknown>;
  theme?: string;
  height?: string;
};

vi.mock("@monaco-editor/react", () => ({
  Editor: ({ value, language, options }: MonacoProps) => (
    <div
      data-testid="editor"
      data-language={language}
      data-value={value ?? ""}
      data-readonly={String(options?.readOnly === true)}
    />
  ),
  DiffEditor: ({ language, original, modified }: DiffProps) => (
    <div
      data-testid="diff-editor"
      data-language={language}
      data-original={original ?? ""}
      data-modified={modified ?? ""}
    />
  ),
}));

function file(path: string, content: string): ArtifactFile {
  return {
    path,
    kind: "file",
    language: "typescript",
    content,
    fullContent: content,
  };
}

type HarnessProps = {
  files?: Map<string, ArtifactFile>;
  activeTab?: string | null;
  diffMode?: boolean;
} & Partial<Pick<ArtifactViewerProps, "saveFile" | "applyPatch" | "closeTab">>;

function Harness({
  files = new Map<string, ArtifactFile>(),
  activeTab = null,
  diffMode = false,
  saveFile = vi.fn(async () => {}),
  applyPatch = vi.fn(async () => {}),
  closeTab = vi.fn(),
}: HarnessProps) {
  const [tab, setTab] = useState<string | null>(activeTab);
  const [diff, setDiff] = useState<boolean>(diffMode);
  return (
    <ArtifactViewer
      files={files}
      activeTab={tab}
      setActiveTab={setTab}
      diffMode={diff}
      setDiffMode={setDiff}
      saveStatus={{}}
      saveFile={saveFile}
      applyPatch={applyPatch}
      closeTab={closeTab}
    />
  );
}

const SINGLE = new Map([["src/main.ts", file("src/main.ts", "export const a = 1;")]]);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("ArtifactViewer", () => {
  it("says so plainly when there are no artifacts at all", () => {
    render(<Harness />);
    expect(
      screen.getByText("No artifacts yet. Run an intent to generate files."),
    ).toBeInTheDocument();
    // Early return path must still have registered the keydown listener.
    expect(() => fireEvent.keyDown(window, { key: "s", ctrlKey: true })).not.toThrow();
  });

  it("asks for a tab selection when files exist but none is active", () => {
    render(<Harness files={SINGLE} />);
    expect(screen.getByText("Select a file tab to preview")).toBeInTheDocument();
    expect(screen.getByText("src/main.ts")).toBeInTheDocument();
    expect(screen.getByTitle("Start Editing")).toBeDisabled();
  });

  it("switches to the clicked tab and shows the file in the editor", () => {
    render(<Harness files={SINGLE} />);
    fireEvent.click(screen.getByText("src/main.ts"));

    const editor = screen.getByTestId("editor");
    expect(editor).toHaveAttribute("data-readonly", "true");
    expect(editor).toHaveAttribute("data-value", "export const a = 1;");
  });

  it("derives the Monaco language from the file extension", () => {
    const files = new Map([
      ["a.rs", file("a.rs", "fn main() {}")],
      ["README.md", file("README.md", "# hi")],
      ["unknown.xyz", file("unknown.xyz", "?")],
    ]);
    render(<Harness files={files} />);

    fireEvent.click(screen.getByText("a.rs"));
    expect(screen.getByTestId("editor")).toHaveAttribute("data-language", "rust");

    fireEvent.click(screen.getByText("README.md"));
    expect(screen.getByTestId("editor")).toHaveAttribute("data-language", "markdown");

    fireEvent.click(screen.getByText("unknown.xyz"));
    expect(screen.getByTestId("editor")).toHaveAttribute("data-language", "plaintext");
  });

  it("toggles into diff mode and renders the diff editor", () => {
    render(<Harness files={SINGLE} activeTab="src/main.ts" diffMode={false} />);

    fireEvent.click(screen.getByTitle("Toggle Diff View"));
    const diff = screen.getByTestId("diff-editor");
    expect(diff).toHaveAttribute("data-language", "typescript");
    expect(diff).toHaveAttribute("data-modified", "export const a = 1;");
    expect(screen.getByTitle("Toggle Diff View")).toHaveTextContent("Code View");

    fireEvent.click(screen.getByTitle("Toggle Diff View"));
    expect(screen.queryByTestId("diff-editor")).not.toBeInTheDocument();
    expect(screen.getByTestId("editor")).toHaveAttribute("data-readonly", "true");
  });

  it("enters edit mode for the active tab and leaves it again", () => {
    render(<Harness files={SINGLE} activeTab="src/main.ts" />);

    fireEvent.click(screen.getByTitle("Start Editing"));
    expect(screen.getByTitle("Exit Editing")).toBeInTheDocument();
    expect(screen.getByTestId("editor")).toHaveAttribute("data-readonly", "false");

    fireEvent.click(screen.getByTitle("Exit Editing"));
    expect(screen.getByTitle("Start Editing")).toBeInTheDocument();
  });

  it("Ctrl+S saves only while editing, and reports success", async () => {
    const saveFile = vi.fn(async () => {});
    render(<Harness files={SINGLE} activeTab="src/main.ts" saveFile={saveFile} />);

    // Not editing -> no save.
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(saveFile).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTitle("Start Editing"));
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });

    await waitFor(() => expect(saveFile).toHaveBeenCalledWith("src/main.ts"));
    expect(await screen.findByText("File saved!")).toBeInTheDocument();
  });

  it("Ctrl+Enter applies the patch only when not editing", async () => {
    const applyPatch = vi.fn(async () => {});
    render(<Harness files={SINGLE} activeTab="src/main.ts" applyPatch={applyPatch} />);

    fireEvent.keyDown(window, { key: "Enter", ctrlKey: true });
    await waitFor(() => expect(applyPatch).toHaveBeenCalledWith("src/main.ts"));
    expect(await screen.findByText("Patch applied!")).toBeInTheDocument();

    // Entering edit mode disables the patch shortcut.
    fireEvent.click(screen.getByTitle("Start Editing"));
    applyPatch.mockClear();
    fireEvent.keyDown(window, { key: "Enter", ctrlKey: true });
    expect(applyPatch).not.toHaveBeenCalled();
  });

  it("does not apply a patch when no tab is active", () => {
    const applyPatch = vi.fn(async () => {});
    render(<Harness files={SINGLE} applyPatch={applyPatch} />);

    fireEvent.keyDown(window, { key: "Enter", ctrlKey: true });
    expect(applyPatch).not.toHaveBeenCalled();
  });

  it("shows the platform-appropriate shortcut legend", () => {
    render(<Harness files={SINGLE} />);
    expect(screen.getByText(/Ctrl\+S save · Ctrl\+⏎ apply/)).toBeInTheDocument();
  });
});
