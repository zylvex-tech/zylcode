import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DesignStudio } from "./DesignStudio";
import { draftProject, type DesignProject, type ProjectSummary } from "./model";
import type { DesignRepository } from "./repository";

// A store with the same shape as both real repositories, so these tests assert
// the workspace's behaviour rather than an environment's quirks.
function fakeRepository() {
  const projects = new Map<string, DesignProject>();
  let seq = 0;
  const exportHtml = vi.fn(async (_id: string, _html: string) => ({
    kind: "file" as const,
    path: "C:/tmp/design/exports/project.html",
  }));

  const repo: DesignRepository = {
    kind: "preview",
    storageNote: "test store",
    list: async (): Promise<ProjectSummary[]> =>
      [...projects.values()]
        .map((p) => ({
          id: p.id,
          name: p.name,
          revision: p.revision,
          updatedAtMs: p.updatedAtMs,
          nodeCount: p.nodes.length,
        }))
        .sort((a, b) => b.updatedAtMs - a.updatedAtMs),
    create: async (name) => {
      const id = `p${++seq}`;
      const project = draftProject(id, name.trim(), Date.now() + seq);
      projects.set(id, project);
      return project;
    },
    load: async (id) => {
      const project = projects.get(id);
      if (!project) throw new Error(`project ${id} is missing`);
      return project;
    },
    save: async (project) => {
      const stored = projects.get(project.id);
      if (!stored) throw new Error(`project ${project.id} is missing`);
      const next = { ...project, revision: stored.revision + 1, updatedAtMs: Date.now() };
      projects.set(next.id, next);
      return next;
    },
    rename: async (id, name) => {
      const stored = projects.get(id);
      if (!stored) throw new Error(`project ${id} is missing`);
      const next = { ...stored, name: name.trim(), revision: stored.revision + 1 };
      projects.set(id, next);
      return next;
    },
    remove: async (id) => {
      projects.delete(id);
    },
    exportHtml,
  };

  return { repo, projects, exportHtml };
}

async function createProject(name: string) {
  fireEvent.change(screen.getByLabelText("New project name"), { target: { value: name } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  await screen.findByText(new RegExp(`Created "${name}"`));
}

describe("Design Studio workspace", () => {
  it("guides a first-time user and creates a project", async () => {
    const { repo, projects } = fakeRepository();
    render(<DesignStudio repository={repo} />);

    expect(await screen.findByText(/No design projects yet/)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Design Studio" })).toBeInTheDocument();

    await createProject("Landing page");

    expect(projects.size).toBe(1);
    expect(screen.getByRole("button", { name: /Landing page/ })).toBeInTheDocument();
    expect(screen.getByText(/Empty canvas/)).toBeInTheDocument();
  });

  it("refuses to create a project without a name", async () => {
    const { repo } = fakeRepository();
    render(<DesignStudio repository={repo} />);
    await screen.findByText(/No design projects yet/);

    fireEvent.click(screen.getByRole("button", { name: "Create project" }));
    expect(await screen.findByText("Enter a project name first.")).toBeInTheDocument();
  });

  it("edits an element, saves it, and reopens the saved document after a restart", async () => {
    const { repo, projects } = fakeRepository();
    const view = render(<DesignStudio repository={repo} />);
    await createProject("Landing page");

    fireEvent.click(screen.getByRole("button", { name: "Add text" }));
    fireEvent.change(screen.getByLabelText("Element text"), { target: { value: "Ship it" } });
    expect(screen.getByTestId("dirty")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await screen.findByText(/Saved "Landing page" as revision 2/);
    expect(screen.queryByTestId("dirty")).toBeNull();

    const saved = [...projects.values()][0];
    expect(saved.nodes[0].text).toBe("Ship it");

    // A fresh component against the same store is what the next app start is.
    view.unmount();
    render(<DesignStudio repository={repo} />);
    fireEvent.click(await screen.findByRole("button", { name: /Landing page/ }));
    await screen.findByText(/Opened "Landing page"/);

    expect(screen.getByText("Ship it")).toBeInTheDocument();
    expect((screen.getByLabelText("Element text") as HTMLInputElement).value).toBe("Ship it");
  });

  it("undoes and redoes edits", async () => {
    const { repo } = fakeRepository();
    render(<DesignStudio repository={repo} />);
    await createProject("History");

    fireEvent.click(screen.getByRole("button", { name: "Add text" }));
    expect((screen.getByLabelText("Element text") as HTMLInputElement).value).toBe("New label");

    fireEvent.change(screen.getByLabelText("Element text"), { target: { value: "Ship it" } });
    fireEvent.click(screen.getByRole("button", { name: "Undo" }));
    expect((screen.getByLabelText("Element text") as HTMLInputElement).value).toBe("New label");

    fireEvent.click(screen.getByRole("button", { name: "Redo" }));
    expect((screen.getByLabelText("Element text") as HTMLInputElement).value).toBe("Ship it");
  });

  it("treats one drag as one undo step", async () => {
    const { repo } = fakeRepository();
    render(<DesignStudio repository={repo} />);
    await createProject("Drag");

    fireEvent.click(screen.getByRole("button", { name: "Add shape" }));
    expect((screen.getByLabelText("Element x") as HTMLInputElement).value).toBe("80");

    const element = screen.getByRole("button", { name: "Element rect" });
    fireEvent.mouseDown(element, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.mouseMove(window, { clientX: 140, clientY: 100 });
    fireEvent.mouseUp(window);

    expect((screen.getByLabelText("Element x") as HTMLInputElement).value).toBe("120");

    fireEvent.click(screen.getByRole("button", { name: "Undo" }));
    expect((screen.getByLabelText("Element x") as HTMLInputElement).value).toBe("80");
  });

  it("validates before saving and reports the reason", async () => {
    const { repo } = fakeRepository();
    render(<DesignStudio repository={repo} />);
    await createProject("Validated");

    fireEvent.click(screen.getByRole("button", { name: "Add text" }));
    fireEvent.change(screen.getByLabelText("Element width"), { target: { value: "0" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(await screen.findByText(/must be between 1 and 100000/)).toBeInTheDocument();
  });

  it("reports a failed save instead of claiming success", async () => {
    const { repo } = fakeRepository();
    repo.save = vi.fn().mockRejectedValue(new Error("disk full"));
    render(<DesignStudio repository={repo} />);
    await createProject("Failing");

    fireEvent.click(screen.getByRole("button", { name: "Add text" }));
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    expect(await screen.findByText(/Save failed: Error: disk full/)).toBeInTheDocument();
    expect(screen.queryByText(/^Saved "/)).toBeNull();
  });

  it("guards unsaved work when switching projects", async () => {
    const { repo } = fakeRepository();
    render(<DesignStudio repository={repo} />);
    await createProject("Alpha");
    await createProject("Beta");

    fireEvent.click(screen.getByRole("button", { name: /Alpha/ }));
    await screen.findByText(/Opened "Alpha"/);
    fireEvent.click(screen.getByRole("button", { name: "Add text" }));

    fireEvent.click(screen.getByRole("button", { name: /Beta/ }));
    expect(await screen.findByText(/Unsaved changes in/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Discard and switch" }));
    await screen.findByText(/Opened "Beta"/);
  });

  it("exports the current document as standalone HTML", async () => {
    const { repo, exportHtml } = fakeRepository();
    render(<DesignStudio repository={repo} />);
    await createProject("Export");

    fireEvent.click(screen.getByRole("button", { name: "Add text" }));
    fireEvent.change(screen.getByLabelText("Element text"), { target: { value: "Hello export" } });
    fireEvent.click(screen.getByRole("button", { name: "Export HTML" }));

    expect(await screen.findByText(/Exported standalone HTML to/)).toBeInTheDocument();
    const [, html] = exportHtml.mock.calls[0];
    expect(html).toContain("Hello export");
    expect(html).toContain("<!doctype html>");
  });

  it("renames and deletes projects, with delete requiring confirmation", async () => {
    const { repo, projects } = fakeRepository();
    render(<DesignStudio repository={repo} />);
    await createProject("Draft");

    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    fireEvent.change(screen.getByLabelText("Rename Draft"), { target: { value: "Final" } });
    fireEvent.click(screen.getByRole("button", { name: "Save name" }));
    await screen.findByText(/Renamed to "Final"/);

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(screen.getByRole("button", { name: "Confirm delete" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Confirm delete" }));

    await screen.findByText("Project deleted.");
    expect(projects.size).toBe(0);
    expect(screen.getByText(/No design projects yet/)).toBeInTheDocument();
  });
});
