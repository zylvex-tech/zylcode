import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useArtifactStream, type ArtifactFile } from "./useArtifactStream";
import type { StreamDelta } from "./events";

// ---------------------------------------------------------------------------
// Artifact stream (lib/useArtifactStream.ts).
//
// This hook turns a raw token stream into editor files. The guarantees under
// test:
//   * Parsing is incremental and lossless across delta boundaries — a block
//     split over several deltas still produces exactly one file, and an
//     incomplete trailing block is buffered rather than dropped.
//   * Language is derived from the file extension when the emitter does not
//     name one, so the editor never highlights a file as plaintext by
//     accident.
//   * Save/patch go through the browser transport with the full content, and
//     a backend refusal lands in an `error` status rather than being shown
//     as `saved`.
//   * Closing the active tab promotes another file instead of leaving the
//     editor with no selection.
// ---------------------------------------------------------------------------

function deltas(...chunks: string[]): StreamDelta[] {
  return chunks.map((delta, index) => ({ index, delta, done: false }));
}

function fileMap(files: Map<string, ArtifactFile>): Map<string, ArtifactFile> {
  return files;
}

const ARTIFACT_BLOCK =
  '<artifact path="src/guard.rs" kind="file" language="rust">fn main() {}</artifact>';

describe("useArtifactStream — parsing", () => {
  it("returns no files before any delta arrives", () => {
    const { result } = renderHook(() => useArtifactStream([]));

    expect(result.current.files.size).toBe(0);
    expect(result.current.activeTab).toBeNull();
  });

  it("parses an artifact block into a file and activates it", async () => {
    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(ARTIFACT_BLOCK) },
    });

    await waitFor(() => expect(result.current.files.size).toBe(1));

    const file = result.current.files.get("src/guard.rs");
    expect(file).toMatchObject({
      path: "src/guard.rs",
      kind: "file",
      language: "rust",
      fullContent: "fn main() {}",
    });
    expect(result.current.activeTab).toBe("src/guard.rs");
  });

  it("reassembles a block that arrives split across deltas", async () => {
    const head = '<artifact path="a.ts" kind="file">';
    const tail = "export const a = 1;</artifact>";

    const { result, rerender } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(head) },
    });
    // The block is incomplete: nothing may be published yet.
    expect(result.current.files.size).toBe(0);

    // The stream appends; the already-consumed prefix must not be consumed twice.
    rerender({ deltas: deltas(head, tail) });

    await waitFor(() => expect(result.current.files.size).toBe(1));
    expect(result.current.files.get("a.ts")?.fullContent).toBe("export const a = 1;");
    expect(fileMap(result.current.files).size).toBe(1);
  });

  it("derives the language from the extension when the tag omits one", async () => {
    const block = '<artifact path="docs/plan.md" kind="file"># plan</artifact>';

    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(block) },
    });

    await waitFor(() => expect(result.current.files.size).toBe(1));
    expect(result.current.files.get("docs/plan.md")?.language).toBe("markdown");
  });

  it("keeps the <content> wrapper's body as the file content", async () => {
    const block = '<artifact path="x.py" kind="file"><content>print("hi")</content></artifact>';

    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(block) },
    });

    await waitFor(() => expect(result.current.files.size).toBe(1));
    expect(result.current.files.get("x.py")?.fullContent).toBe('print("hi")');
  });

  it("unwraps CDATA content so markers never reach the editor buffer", async () => {
    const block = '<artifact path="y.ts" kind="file"><![CDATA[const y = 2;]]></artifact>';

    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(block) },
    });

    await waitFor(() => expect(result.current.files.size).toBe(1));
    expect(result.current.files.get("y.ts")?.fullContent).toBe("const y = 2;");
  });

  it("falls back to a kind-derived path when the tag names no path", async () => {
    const block = '<artifact kind="diff">--- a/file</artifact>';

    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(block) },
    });

    await waitFor(() => expect(result.current.files.size).toBe(1));
    expect(result.current.files.has("artifact.diff")).toBe(true);
  });

  it("converts a fenced code block into a snippet file", async () => {
    const block = "```rust\nfn tagged() {}\n```";

    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(block) },
    });

    await waitFor(() => expect(result.current.files.size).toBe(1));

    const file = result.current.files.get("snippet.rust");
    expect(file).toMatchObject({ kind: "Fence", language: "rust" });
    expect(file?.fullContent).toBe("fn tagged() {}");
  });

  it("names an unlabelled fence plaintext", async () => {
    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas("```\nbare\n```") },
    });

    await waitFor(() => expect(result.current.files.has("snippet.plaintext")).toBe(true));
  });

  it("ignores deltas with no content", async () => {
    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: [{ index: 0, delta: "", done: true }] },
    });

    expect(result.current.files.size).toBe(0);
  });
});

describe("useArtifactStream — tab lifecycle", () => {
  it("keeps the selected tab when a second file arrives", async () => {
    const { result, rerender } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(ARTIFACT_BLOCK) },
    });
    await waitFor(() => expect(result.current.activeTab).toBe("src/guard.rs"));

    rerender({
      deltas: deltas(
        ARTIFACT_BLOCK,
        '<artifact path="src/second.ts" kind="file">export {}</artifact>',
      ),
    });

    await waitFor(() => expect(result.current.files.size).toBe(2));
    expect(result.current.activeTab).toBe("src/guard.rs");
  });

  it("promotes another file when the active tab is closed", async () => {
    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: {
        deltas: deltas(
          ARTIFACT_BLOCK + '<artifact path="src/second.ts" kind="file">export {}</artifact>',
        ),
      },
    });
    await waitFor(() => expect(result.current.files.size).toBe(2));

    act(() => result.current.closeTab("src/guard.rs"));

    expect(result.current.files.has("src/guard.rs")).toBe(false);
    expect(result.current.activeTab).toBe("src/second.ts");
  });

  it("clears the active tab when the last file closes", async () => {
    const { result } = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(ARTIFACT_BLOCK) },
    });
    await waitFor(() => expect(result.current.files.size).toBe(1));

    act(() => result.current.closeTab("src/guard.rs"));

    expect(result.current.files.size).toBe(0);
    expect(result.current.activeTab).toBeNull();
  });

  it("can toggle diff mode", async () => {
    const { result } = renderHook(() => useArtifactStream([]));

    expect(result.current.diffMode).toBe(false);
    act(() => result.current.setDiffMode(true));
    expect(result.current.diffMode).toBe(true);
  });
});

describe("useArtifactStream — save and patch", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  async function withParsedFile() {
    const rendered = renderHook(({ deltas: d }) => useArtifactStream(d), {
      initialProps: { deltas: deltas(ARTIFACT_BLOCK) },
    });
    await waitFor(() => expect(rendered.result.current.files.size).toBe(1));
    return rendered;
  }

  it("posts the full content to the save endpoint", async () => {
    fetchMock.mockResolvedValue({ ok: true, json: async () => ({ saved: true }) });
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.saveFile("src/guard.rs");
    });

    expect(fetchMock).toHaveBeenCalledWith("/api/artifact/save", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ label: "src/guard.rs", content: "fn main() {}" }),
    });
    expect(result.current.saveStatus["src/guard.rs"]).toBe("saved");
  });

  it("returns to idle after the confirmation window", async () => {
    fetchMock.mockResolvedValue({ ok: true, json: async () => ({ saved: true }) });
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.saveFile("src/guard.rs");
    });
    expect(result.current.saveStatus["src/guard.rs"]).toBe("saved");

    // The confirmation is a real 1.5s timer; let it elapse rather than
    // faking timers, which would also freeze React's scheduler here.
    await waitFor(() => expect(result.current.saveStatus["src/guard.rs"]).toBe("idle"), {
      timeout: 3000,
    });
  });

  it("records an error status when the backend refuses", async () => {
    fetchMock.mockResolvedValue({
      ok: false,
      status: 422,
      json: async () => ({}),
    });
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.saveFile("src/guard.rs");
    });

    expect(result.current.saveStatus["src/guard.rs"]).toBe("error");
  });

  it("records an error status when the backend answers with an error envelope", async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({ error: "path escapes the workspace root" }),
    });
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.saveFile("src/guard.rs");
    });

    expect(result.current.saveStatus["src/guard.rs"]).toBe("error");
  });

  it("does not mark a file saved when the transport throws", async () => {
    fetchMock.mockRejectedValue(new Error("offline"));
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.saveFile("src/guard.rs");
    });

    expect(result.current.saveStatus["src/guard.rs"]).toBe("error");
  });

  it("ignores a save for a path that was never parsed", async () => {
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.saveFile("nope.ts");
    });

    expect(fetchMock).not.toHaveBeenCalled();
    expect(result.current.saveStatus["nope.ts"]).toBeUndefined();
  });

  it("posts the file content as a patch", async () => {
    fetchMock.mockResolvedValue({ ok: true, json: async () => ({ applied: true }) });
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.applyPatch("src/guard.rs");
    });

    expect(fetchMock).toHaveBeenCalledWith("/api/artifact/patch", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ patch: "fn main() {}" }),
    });
    expect(result.current.saveStatus["src/guard.rs"]).toBe("saved");
  });

  it("records an error when the patch is refused", async () => {
    fetchMock.mockResolvedValue({ ok: false, status: 409, json: async () => ({}) });
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.applyPatch("src/guard.rs");
    });

    expect(result.current.saveStatus["src/guard.rs"]).toBe("error");
  });

  it("ignores a patch for an unknown path", async () => {
    const { result } = await withParsedFile();

    await act(async () => {
      await result.current.applyPatch("nope.ts");
    });

    expect(fetchMock).not.toHaveBeenCalled();
  });
});
