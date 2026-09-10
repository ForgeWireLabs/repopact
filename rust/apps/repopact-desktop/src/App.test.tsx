import { render, screen } from "@testing-library/react";
import * as axe from "axe-core";
import { vi } from "vitest";
import App, { PlanDialog } from "./App";
import type { MutationPlanView } from "./generated/types";

vi.mock("./lib/api", () => ({
  desktopApi: {
    listenForChanges: vi.fn().mockResolvedValue(() => undefined),
    selectRepository: vi.fn(),
    overview: vi.fn(),
    workItems: vi.fn(),
    decisions: vi.fn(),
    evidence: vi.fn(),
  },
}));

describe("workbench shell", () => {
  it("renders a keyboard-addressable empty repository state", () => {
    render(<App />);
    expect(screen.getByRole("navigation", { name: "Workbench sections" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Select a repository to begin" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Choose repository" })).toBeEnabled();
  });

  it("keeps plan review explicit and accessible", async () => {
    const plan: MutationPlanView = {
      session_id: "session-1",
      plan_handle: "plan-1-1",
      plan_token: "token",
      intent: { kind: "transition_work_item", payload: { id: "001", status: "blocked" } },
      diagnostics: [],
      generated_impacts: [],
      graph_impacts: [],
      preview: "rename work/active/001-item work/blocked/001-item",
      applicable: true,
    };
    const { container } = render(<PlanDialog plan={plan} onApply={vi.fn()} onDiscard={vi.fn()} busy={false} />);
    expect(screen.getByRole("dialog", { name: "Review proposed mutation" })).toBeInTheDocument();
    expect(screen.getByText("Opaque plan handle")).toBeInTheDocument();
    expect((await axe.run(container)).violations).toEqual([]);
  });
});
