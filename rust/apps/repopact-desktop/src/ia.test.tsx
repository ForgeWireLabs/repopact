import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import App, { LocalPager, SectionTabs } from "./App";
import { desktopApi } from "./lib/api";
import type {
  AnalysisView,
  DecisionSummaryView,
  EvidenceSummaryView,
  GraphView,
  RepositoryOverview,
  ValidationView,
  WorkItemDetailView,
  WorkItemSummaryView,
} from "./generated/types";

vi.mock("./lib/api", () => ({
  desktopApi: {
    listenForChanges: vi.fn().mockResolvedValue(() => undefined),
    selectRepository: vi.fn(),
    closeRepository: vi.fn(),
    overview: vi.fn(),
    refresh: vi.fn(),
    validate: vi.fn(),
    workItems: vi.fn(),
    workItem: vi.fn(),
    decisions: vi.fn(),
    decision: vi.fn(),
    evidence: vi.fn(),
    evidenceRecord: vi.fn(),
    graph: vi.fn(),
    analyze: vi.fn(),
    plan: vi.fn(),
    apply: vi.fn(),
    discard: vi.fn(),
  },
}));

const overview: RepositoryOverview = {
  session_id: "session-1",
  generation: 1,
  identity: { root: "C:/RepoPact", git_common_dir: null, git_worktree_root: null, linked_worktree: false },
  validation: { valid: true, diagnostics: [], error_count: 0, warning_count: 0 },
  work_item_count: 15,
  evidence_count: 1,
  decision_count: 1,
  graph_node_count: 1,
  graph_edge_count: 0,
  snapshot_token: "snapshot-1",
  watcher: { state: "running", recursive: true, debounce_ms: 150 },
};

const workItems: WorkItemSummaryView[] = [
  { id: "001", title: "Blocked item", status: "blocked", owner_scope: "governance", affected_scopes: [], depends_on: [], provenance: "concrete", path: "work/active/001/work-item.json", criterion_count: 0, evidence_count: 0 },
  { id: "002", title: "Active item", status: "active", owner_scope: "governance", affected_scopes: [], depends_on: [], provenance: "concrete", path: "work/active/002/work-item.json", criterion_count: 0, evidence_count: 0 },
  { id: "003", title: "Proposed item", status: "proposed", owner_scope: "governance", affected_scopes: [], depends_on: [], provenance: "concrete", path: "work/proposed/003/work-item.json", criterion_count: 0, evidence_count: 0 },
  { id: "004", title: "Deferred item", status: "deferred", owner_scope: "governance", affected_scopes: [], depends_on: [], provenance: "concrete", path: "work/deferred/004/work-item.json", criterion_count: 0, evidence_count: 0 },
  ...Array.from({ length: 12 }, (_, index) => ({
    id: `C${String(index + 1).padStart(2, "0")}`,
    title: `Complete item ${index + 1}`,
    status: "completed",
    owner_scope: "governance",
    affected_scopes: [],
    depends_on: [],
    provenance: "concrete",
    path: `work/completed/C${String(index + 1).padStart(2, "0")}/work-item.json`,
    criterion_count: 0,
    evidence_count: 0,
  } satisfies WorkItemSummaryView)),
];

const decisions: DecisionSummaryView[] = [{
  reference: { kind: "decision", id: "0042", path: "decisions/0042-canonical-rust-engine-and-versioned-stdio-compatibility.md" },
  readable: true,
  title: "Canonical Rust engine",
  status: "accepted",
  date: "2026-09-01",
  supersedes: [],
}];

const evidence: EvidenceSummaryView[] = [{
  reference: { kind: "evidence_run", id: "run-1", path: "evidence/runs/run-1.json" },
  readable: true,
  timestamp: "2026-09-10T10:00:00Z",
  work_item: "058",
  result: "passed",
  provenance: "test",
}];

const validation: ValidationView = { valid: true, diagnostics: [], error_count: 0, warning_count: 0 };
const graph: GraphView = { nodes: [], edges: [] };
const analysis: AnalysisView = { findings: [] };
const workDetail: WorkItemDetailView = {
  summary: workItems[0],
  work_item: {
    id: "001",
    title: "Blocked item",
    status: "blocked",
    owner_scope: "governance",
    affected_scopes: [],
    depends_on: [],
    provenance: "concrete",
    preflight: null,
    acceptance_criteria: [],
    created: "2026-09-10",
    updated: "2026-09-10",
  },
  dependents: [],
  raw_record: {},
};

function configureApi() {
  vi.mocked(desktopApi.selectRepository).mockResolvedValue(overview);
  vi.mocked(desktopApi.workItems).mockResolvedValue(workItems);
  vi.mocked(desktopApi.decisions).mockResolvedValue(decisions);
  vi.mocked(desktopApi.evidence).mockResolvedValue(evidence);
  vi.mocked(desktopApi.validate).mockResolvedValue(validation);
  vi.mocked(desktopApi.graph).mockResolvedValue(graph);
  vi.mocked(desktopApi.analyze).mockResolvedValue(analysis);
  vi.mocked(desktopApi.workItem).mockResolvedValue(workDetail);
}

describe("adaptive workbench information architecture", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    configureApi();
  });

  it("exposes exact lifecycle tabs and keeps blocked work inside Active", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Choose repository" }));
    await screen.findByRole("button", { name: "Work" });
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    await screen.findByRole("heading", { name: "Work items" });

    const tabs = screen.getAllByRole("tab");
    expect(tabs.slice(0, 4).map((tab) => tab.textContent?.replace(/\d+/g, "").trim())).toEqual(["Proposed", "Active", "Deferred", "Complete"]);
    expect(screen.getByText("Blocked")).toBeInTheDocument();
    expect(screen.getByText("blocked · governance")).toBeInTheDocument();
  });

  it("pages locally and does not call the backend for IA changes", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Choose repository" }));
    await screen.findByRole("button", { name: "Work" });
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    await screen.findByRole("heading", { name: "Work items" });
    fireEvent.click(screen.getByRole("tab", { name: /Complete/ }));
    expect(screen.getByText("1–11 of 12")).toBeInTheDocument();

    const callsAfterSnapshot = {
      work: vi.mocked(desktopApi.workItems).mock.calls.length,
      decisions: vi.mocked(desktopApi.decisions).mock.calls.length,
      evidence: vi.mocked(desktopApi.evidence).mock.calls.length,
      validation: vi.mocked(desktopApi.validate).mock.calls.length,
      graph: vi.mocked(desktopApi.graph).mock.calls.length,
      analysis: vi.mocked(desktopApi.analyze).mock.calls.length,
    };
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByText("12–12 of 12")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Dashboard" }));
    fireEvent.click(screen.getByRole("button", { name: /Graph edges/ }));
    await waitFor(() => {
      expect(vi.mocked(desktopApi.workItems).mock.calls.length).toBe(callsAfterSnapshot.work);
      expect(vi.mocked(desktopApi.decisions).mock.calls.length).toBe(callsAfterSnapshot.decisions);
      expect(vi.mocked(desktopApi.evidence).mock.calls.length).toBe(callsAfterSnapshot.evidence);
      expect(vi.mocked(desktopApi.validate).mock.calls.length).toBe(callsAfterSnapshot.validation);
      expect(vi.mocked(desktopApi.graph).mock.calls.length).toBe(callsAfterSnapshot.graph);
      expect(vi.mocked(desktopApi.analyze).mock.calls.length).toBe(callsAfterSnapshot.analysis);
    });
  });

  it("renders keyboard-addressable tabs and bounded pager controls", () => {
    const onTab = vi.fn();
    const onPage = vi.fn();
    render(<><SectionTabs tabs={[{ id: "one", label: "One" }, { id: "two", label: "Two" }]} value="one" onChange={onTab} label="Test views" panelId="test-panel" /><LocalPager page={0} pageSize={7} total={8} compact onPageChange={onPage} /></>);
    const first = screen.getByRole("tab", { name: "One" });
    fireEvent.keyDown(first, { key: "ArrowRight" });
    expect(onTab).toHaveBeenCalledWith("two");
    expect(screen.getByText("1–7 of 8")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Previous" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(onPage).toHaveBeenCalledWith(1);
  });

  it("remembers page tabs during navigation and resets them for a new repository", async () => {
    const secondOverview = { ...overview, session_id: "session-2", generation: 1, identity: { ...overview.identity, root: "C:/Scratch" } };
    vi.mocked(desktopApi.selectRepository).mockResolvedValueOnce(overview).mockResolvedValueOnce(secondOverview);
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Choose repository" }));
    await screen.findByRole("heading", { name: "Dashboard" });
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    await screen.findByRole("heading", { name: "Work items" });
    fireEvent.click(screen.getByRole("tab", { name: /Complete/ }));
    fireEvent.click(screen.getByRole("button", { name: "Dashboard" }));
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    expect(screen.getByRole("tab", { name: /Complete/ })).toHaveAttribute("aria-selected", "true");
    fireEvent.click(screen.getByRole("button", { name: "Switch repository" }));
    await waitFor(() => {
      expect(vi.mocked(desktopApi.selectRepository)).toHaveBeenCalledTimes(2);
      expect(screen.getByRole("tab", { name: /Active/ })).toHaveAttribute("aria-selected", "true");
    });
  });

  it("uses compact navigation and list-to-detail drill-in without another snapshot", async () => {
    vi.stubGlobal("matchMedia", (query: string) => ({
      matches: query.includes("max-width: 900px"),
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Choose repository" }));
    await screen.findByRole("heading", { name: "Dashboard" });
    fireEvent.click(screen.getByRole("button", { name: "Open workbench navigation" }));
    fireEvent.click(screen.getByRole("button", { name: "Work" }));
    await screen.findByRole("heading", { name: "Work items" });
    fireEvent.click(screen.getByRole("button", { name: /001 · Blocked item/ }));
    await screen.findByRole("heading", { name: "Blocked item" });
    expect(screen.getByRole("button", { name: /Back to Active/ })).toBeInTheDocument();
    expect(vi.mocked(desktopApi.workItems)).toHaveBeenCalledTimes(1);
  });
});
