import { afterEach, describe, expect, it, vi } from "vitest";
import { desktopRepository, previewRepository } from "./repository";
import { draftProject, type DesignProject } from "./model";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

afterEach(() => {
  localStorage.clear();
  invoke.mockReset();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("preview repository (browser localStorage)", () => {
  it("persists projects across repository instances, the way a reload does", async () => {
    const first = previewRepository();
    const created = await first.create("Landing page");

    const second = previewRepository();
    const listed = await second.list();
    expect(listed).toHaveLength(1);
    expect(listed[0].name).toBe("Landing page");

    const loaded = await second.load(created.id);
    expect(loaded.nodes).toEqual([]);
  });

  it("advances the revision on save and refuses a stale document", async () => {
    const repo = previewRepository();
    const created = await repo.create("Board");

    const edited: DesignProject = { ...created, nodes: [] };
    const saved = await repo.save(edited);
    expect(saved.revision).toBe(created.revision + 1);
    expect(saved.createdAtMs).toBe(created.createdAtMs);

    await expect(repo.save(edited)).rejects.toThrow(/changed since it was opened/);
  });

  it("refuses invalid documents before they are stored", async () => {
    const repo = previewRepository();
    const created = await repo.create("Validated");
    await expect(repo.save({ ...created, name: "  " })).rejects.toThrow(/name is required/);
    await expect(repo.create("   ")).rejects.toThrow(/name is required/);
  });

  it("renames, removes and reports missing projects", async () => {
    const repo = previewRepository();
    const created = await repo.create("Draft");
    const renamed = await repo.rename(created.id, " Final ");
    expect(renamed.name).toBe("Final");

    await repo.remove(created.id);
    expect(await repo.list()).toEqual([]);
    await expect(repo.load(created.id)).rejects.toThrow(/not in preview storage/);
    await expect(repo.rename(created.id, "x")).rejects.toThrow(/not in preview storage/);
    await expect(repo.remove("../escape")).rejects.toThrow(/not a valid project id/);
  });

  it("downloads an export when the environment supports blobs", async () => {
    const repo = previewRepository();
    const created = await repo.create("Export");
    vi.stubGlobal("URL", {
      createObjectURL: vi.fn(() => "blob:design"),
      revokeObjectURL: vi.fn(),
    });
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});

    const outcome = await repo.exportHtml(created.id, "<!doctype html>");
    expect(outcome).toEqual({ kind: "download", filename: "design-export.html" });
    expect(click).toHaveBeenCalledOnce();
  });

  it("says so when it cannot download", async () => {
    const repo = previewRepository();
    const created = await repo.create("Export");
    vi.stubGlobal("URL", {});

    const outcome = await repo.exportHtml(created.id, "<!doctype html>");
    expect(outcome.kind).toBe("unavailable");
  });
});

describe("desktop repository (Tauri IPC)", () => {
  it("uses the exact command names and argument shapes the Rust side registers", async () => {
    const repo = desktopRepository("TAURI_DESKTOP");
    const project = draftProject("p1", "Desktop", 0);

    invoke.mockResolvedValueOnce([]);
    await repo.list();
    expect(invoke).toHaveBeenLastCalledWith("design_list", undefined);

    invoke.mockResolvedValueOnce(project);
    await repo.create("Desktop");
    expect(invoke).toHaveBeenLastCalledWith("design_create", { name: "Desktop" });

    invoke.mockResolvedValueOnce(project);
    await repo.load("p1");
    expect(invoke).toHaveBeenLastCalledWith("design_load", { id: "p1" });

    invoke.mockResolvedValueOnce({ ...project, revision: 2 });
    await repo.save(project);
    expect(invoke).toHaveBeenLastCalledWith("design_save", { project });

    invoke.mockResolvedValueOnce(project);
    await repo.rename("p1", "Renamed");
    expect(invoke).toHaveBeenLastCalledWith("design_rename", { id: "p1", name: "Renamed" });

    invoke.mockResolvedValueOnce(null);
    await repo.remove("p1");
    expect(invoke).toHaveBeenLastCalledWith("design_delete", { id: "p1" });

    invoke.mockResolvedValueOnce("C:/data/design-projects/exports/p1.html");
    await expect(repo.exportHtml("p1", "<!doctype html>")).resolves.toEqual({
      kind: "file",
      path: "C:/data/design-projects/exports/p1.html",
    });
    expect(invoke).toHaveBeenLastCalledWith("design_export", {
      id: "p1",
      html: "<!doctype html>",
    });
  });

  it("surfaces a failing command as an error instead of an empty result", async () => {
    const repo = desktopRepository("TAURI_DESKTOP");
    invoke.mockRejectedValueOnce(new Error("store unavailable"));
    await expect(repo.list()).rejects.toThrow(/store unavailable/);
  });
});
