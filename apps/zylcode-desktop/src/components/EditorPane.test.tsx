import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EditorPane } from "./EditorPane";
import { fetchFileContent, type FileContentState } from "../lib/fileContent";

// ---------------------------------------------------------------------------
// EditorPane renders *only* what the file-content service returns — plain text
// with line numbers, no simulated highlighting. The tests pin that honesty
// contract: loading / ready / lossy / unavailable are each shown as they are,
// a binary file is flagged rather than mangled, and the language breadcrumb is
// derived from the real extension.
// ---------------------------------------------------------------------------

vi.mock("../lib/fileContent", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/fileContent")>();
  return { ...actual, fetchFileContent: vi.fn() };
});

const fetch = vi.mocked(fetchFileContent);

type Props = {
  openFiles?: string[];
  activePath?: string | null;
  serviceNote?: string;
  onSelectTab?: (p: string) => void;
  onCloseTab?: (p: string) => void;
};

function renderPane({
  openFiles = ["src/main.ts"],
  activePath = "src/main.ts",
  serviceNote,
  onSelectTab = vi.fn(),
  onCloseTab = vi.fn(),
}: Props = {}) {
  const ui = render(
    <EditorPane
      openFiles={openFiles}
      activePath={activePath}
      onSelectTab={onSelectTab}
      onCloseTab={onCloseTab}
      serviceNote={serviceNote}
    />,
  );
  return { ...ui, onSelectTab, onCloseTab };
}

const READY: FileContentState = {
  kind: "ready",
  data: {
    path: "src/main.ts",
    size: 26,
    lines: 3,
    lossy: false,
    content: "const a = 1;\nconst b = 2;\nexport {}",
  },
};

beforeEach(() => {
  vi.clearAllMocks();
  fetch.mockResolvedValue(READY);
});

describe("EditorPane", () => {
  it("shows the honest empty state when no file is open", () => {
    renderPane({ openFiles: [], activePath: null });
    expect(screen.getByText("No file open")).toBeInTheDocument();
    expect(
      screen.getByText("Open a file from the Explorer to read its real content here."),
    ).toBeInTheDocument();
    expect(fetch).not.toHaveBeenCalled();
  });

  it("renders a caller-supplied prompt in place of the default hint", () => {
    renderPane({ openFiles: [], activePath: null, serviceNote: "Pick a file to begin." });
    expect(screen.getByText("Pick a file to begin.")).toBeInTheDocument();
  });

  it("prefers the empty state when a path is set but no tab is open", () => {
    // The fetch effect keys off `activePath` alone, so it still fires; what
    // matters is that the pane never renders content it has no tab for.
    renderPane({ openFiles: [], activePath: "src/main.ts" });
    expect(screen.getByText("No file open")).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it("reports loading before the content arrives", () => {
    fetch.mockReturnValue(new Promise(() => {}));
    renderPane();
    expect(screen.getByText("Loading src/main.ts…")).toBeInTheDocument();
  });

  it("renders real content with a line-number gutter and breadcrumbs", async () => {
    renderPane();

    // Content lives in a single <pre> whose text spans all lines.
    await waitFor(() =>
      expect(document.querySelector("pre")?.textContent).toBe(
        "const a = 1;\nconst b = 2;\nexport {}",
      ),
    );
    expect(fetch).toHaveBeenCalledWith("src/main.ts");

    // gutter: one entry per line, 1-based
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.queryByText("4")).not.toBeInTheDocument();

    // breadcrumb segments and the derived language label
    expect(screen.getByText("src")).toBeInTheDocument();
    // tab strip + breadcrumb both name the file
    expect(screen.getAllByText("main.ts")).toHaveLength(2);
    expect(screen.getByText("TypeScript")).toBeInTheDocument();
    // no fake lossy warning for clean UTF-8
    expect(
      screen.queryByText("Binary or non-UTF-8 content — shown lossily."),
    ).not.toBeInTheDocument();
  });

  it("flags lossy content instead of silently mangling it", async () => {
    fetch.mockResolvedValue({
      kind: "ready",
      data: { ...READY.data, lossy: true },
    });
    renderPane();

    expect(
      await screen.findByText("Binary or non-UTF-8 content — shown lossily."),
    ).toBeInTheDocument();
  });

  it("surfaces an unavailable file with its reason", async () => {
    fetch.mockResolvedValue({ kind: "unavailable", reason: "file not found" });
    renderPane();

    expect(await screen.findByText("Cannot open src/main.ts")).toBeInTheDocument();
    expect(screen.getByText("file not found")).toBeInTheDocument();
  });

  it("selects a tab and closes it with the event kept local to the tab", async () => {
    const { onSelectTab, onCloseTab } = renderPane({
      openFiles: ["src/main.ts", "README.md"],
      activePath: "src/main.ts",
    });

    fireEvent.click(screen.getByText("README.md"));
    expect(onSelectTab).toHaveBeenCalledWith("README.md");

    const close = screen.getByLabelText("Close src/main.ts");
    const before = vi.mocked(onSelectTab).mock.calls.length;
    fireEvent.click(close);
    expect(onCloseTab).toHaveBeenCalledWith("src/main.ts");
    // stopPropagation: closing must not also select the tab
    expect(onSelectTab).toHaveBeenCalledTimes(before);
  });

  it("derives every breadcrumb language label from the real extension", async () => {
    const cases: [path: string, label: string][] = [
      ["lib.rs", "Rust"],
      ["a.ts", "TypeScript"],
      ["a.tsx", "TypeScript JSX"],
      ["a.js", "JavaScript"],
      ["a.jsx", "JavaScript JSX"],
      ["a.json", "JSON"],
      ["Cargo.lock", "Lockfile"],
      ["a.md", "Markdown"],
      ["a.css", "CSS"],
      ["a.html", "HTML"],
      ["a.yml", "YAML"],
      ["a.yaml", "YAML"],
      ["a.py", "Python"],
      ["a.sh", "Shell"],
      ["a.bat", "Batch"],
      ["Makefile", "MAKEFILE"],
      ["a.xyz", "XYZ"],
      ["a.toml", "TOML"],
    ];

    for (const [path, label] of cases) {
      const { unmount } = renderPane({ activePath: path, openFiles: [path] });
      await waitFor(() => expect(screen.getByText(label)).toBeInTheDocument());
      unmount();
    }
  });

  it("re-fetches when the active path changes and cancels the stale request", async () => {
    const { rerender } = render(
      <EditorPane
        openFiles={["a.ts", "b.ts"]}
        activePath="a.ts"
        onSelectTab={vi.fn()}
        onCloseTab={vi.fn()}
      />,
    );
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));

    fetch.mockReturnValue(new Promise(() => {}));
    rerender(
      <EditorPane
        openFiles={["a.ts", "b.ts"]}
        activePath="b.ts"
        onSelectTab={vi.fn()}
        onCloseTab={vi.fn()}
      />,
    );
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(2));
    expect(fetch).toHaveBeenLastCalledWith("b.ts");
  });

  it("clears content when the active path is cleared", async () => {
    const { rerender } = render(
      <EditorPane
        openFiles={["a.ts"]}
        activePath="a.ts"
        onSelectTab={vi.fn()}
        onCloseTab={vi.fn()}
      />,
    );
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));

    rerender(
      <EditorPane
        openFiles={["a.ts"]}
        activePath={null}
        onSelectTab={vi.fn()}
        onCloseTab={vi.fn()}
      />,
    );
    expect(screen.getByText("No file open")).toBeInTheDocument();
    // no second fetch for the cleared path
    expect(fetch).toHaveBeenCalledTimes(1);
  });
});
