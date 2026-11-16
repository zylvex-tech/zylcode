import { describe, expect, it } from "vitest";
import { exportFileName, renderProjectHtml } from "./export";
import { draftProject, newNode } from "./model";

describe("standalone HTML export", () => {
  it("renders the canvas and its elements with no external references", () => {
    const project = draftProject("p1", "Landing <page>", 0);
    project.canvas.width = 640;
    project.canvas.height = 480;
    project.nodes.push(
      { ...newNode("text", 0), text: "Ship it", x: 12, y: 24 },
      { ...newNode("rect", 1), color: "#3b82f6" },
    );

    const html = renderProjectHtml(project);

    expect(html).toContain("<!doctype html>");
    expect(html).toContain("width:640px;height:480px");
    expect(html).toContain("Ship it");
    expect(html).toContain("left:12px;top:24px");
    expect(html).toContain("background:#3b82f6");
    expect(html).toContain("<title>Landing &lt;page&gt;</title>");
    expect(html).not.toMatch(/https?:\/\//);
    expect(html).not.toContain("<script");
  });

  it("escapes element text so a design cannot inject markup", () => {
    const project = draftProject("p1", "Safe", 0);
    project.nodes.push({
      ...newNode("text", 0),
      text: `<img src=x onerror="alert('x')"> & "quoted"`,
    });

    const html = renderProjectHtml(project);

    expect(html).not.toContain("<img");
    expect(html).toContain("&lt;img");
    expect(html).toContain("&quot;quoted&quot;");
    expect(html).toContain("&amp;");
  });

  it("derives a safe file name from the project name", () => {
    expect(exportFileName(draftProject("p1", "Landing Page / v2!", 0))).toBe(
      "landing-page-v2.html",
    );
    expect(exportFileName(draftProject("p2", "   ", 0))).toBe("design.html");
  });
});
