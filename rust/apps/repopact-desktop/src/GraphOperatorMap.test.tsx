import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { GraphOperatorMap } from "./GraphOperatorMap";
import { desktopApi } from "./lib/api";
import type { GraphStatusView } from "./generated/types";

// ROG-027 (Decision 0052): focused tests for the operator repository map.
// Every assertion here proves the component consumes the canonical typed
// query boundary -- never that it invents its own traversal/search/
// impact semantics.

vi.mock("./lib/api", () => ({
  desktopApi: {
    graphQuery: vi.fn(),
    graphStatus: vi.fn(),
    graphVerify: vi.fn(),
    graphBuild: vi.fn(),
  },
}));

const freshStatus: GraphStatusView = {
  freshness: "fresh",
  capability_state: "explicit_enabled",
  manifest: {
    graph_schema_version: 3,
    generator_version: "test",
    source_projection_fingerprint: "fp",
    node_count: 2,
    edge_count: 1,
    shard_count: 16,
    node_shards: [],
    edge_shards: [],
    coverage: { nodes_by_layer: {}, edges_by_layer: {} },
    excluded_policy_id: "policy",
  },
  diagnostics: [],
};

function envelope<T>(result: T, overrides: Partial<Record<string, unknown>> = {}) {
  return {
    query_contract_version: 1,
    graph_schema_version: 3,
    graph_fingerprint: "fp",
    status: { basis: "durable", durable_freshness: "fresh", coverage: "complete" },
    warnings: [],
    truncated: false,
    returned_nodes: 0,
    returned_edges: 0,
    next_cursor: null,
    result,
    ...overrides,
  };
}

const fileFact = { id: "file:src/lib.rs", kind: "file", label: "lib.rs", layer: "physical", source: { kind: "file", id: "src/lib.rs", path: "src/lib.rs" } };
const workFact = { id: "work:100", kind: "work_item", label: "Fixture", layer: "governance", source: { kind: "work_item", id: "100", path: "work/active/100/work-item.json" } };

beforeEach(() => {
  vi.mocked(desktopApi.graphStatus).mockResolvedValue(freshStatus);
});

it("renders the freshness/coverage banner distinctly from a fabricated fresh-durable claim", async () => {
  vi.mocked(desktopApi.graphStatus).mockResolvedValue({
    ...freshStatus,
    capability_state: "enabled_missing",
    freshness: "corrupt",
  });
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  const banner = await screen.findByTestId("graph-freshness-banner");
  expect(banner).toHaveTextContent("enabled but missing");
});

it("search is bounded and typed: a query.search request is issued, never a client-side scan", async () => {
  vi.mocked(desktopApi.graphQuery).mockResolvedValue(envelope({ query: "src/lib.rs", matches: [{ node: fileFact, rank: "exact", matched_field: "repository_relative_path" }] }));
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "src/lib.rs" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  await waitFor(() => expect(desktopApi.graphQuery).toHaveBeenCalledWith(expect.objectContaining({ operation: "search", text: "src/lib.rs" })));
  expect(await screen.findAllByTestId("graph-search-result")).toHaveLength(1);
});

it("ambiguous/multiple results do not auto-select a node", async () => {
  vi.mocked(desktopApi.graphQuery).mockResolvedValue(envelope({
    query: "work",
    matches: [
      { node: fileFact, rank: "substring", matched_field: "label" },
      { node: workFact, rank: "substring", matched_field: "label" },
    ],
  }));
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "work" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  await waitFor(() => expect(screen.getAllByTestId("graph-search-result")).toHaveLength(2));
  expect(screen.queryByTestId("graph-detail-identity")).not.toBeInTheDocument();
});

it("selecting a node loads bounded identity/containment facts via graph.context", async () => {
  vi.mocked(desktopApi.graphQuery).mockImplementation(async (request) => {
    if (request.operation === "search") return envelope({ query: "src/lib.rs", matches: [{ node: fileFact, rank: "exact", matched_field: "repository_relative_path" }] });
    if (request.operation === "context") return envelope({ identity: fileFact, containment: [], direct_relations: [{ from: "file:src/lib.rs", to: "work:100", kind: "supports_work_item", layer: "governance", derivation: "canonical_record", source: workFact.source, provenance: "persisted" }], related_nodes: [] });
    throw new Error(`unexpected operation ${request.operation}`);
  });
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "src/lib.rs" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  fireEvent.click(await screen.findByTestId("graph-search-result"));
  const identity = await screen.findByTestId("graph-detail-identity");
  expect(identity).toHaveTextContent("supports_work_item");
});

it("layer filters change the underlying neighbors request, not merely hide an already-fetched list", async () => {
  vi.mocked(desktopApi.graphQuery).mockImplementation(async (request) => {
    if (request.operation === "search") return envelope({ query: "src/lib.rs", matches: [{ node: fileFact, rank: "exact", matched_field: "repository_relative_path" }] });
    if (request.operation === "context") return envelope({ identity: fileFact, containment: [], direct_relations: [], related_nodes: [] });
    if (request.operation === "neighbors") return envelope({ node: fileFact, relations: [], neighbors: [] });
    throw new Error(`unexpected operation ${request.operation}`);
  });
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "src/lib.rs" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  fireEvent.click(await screen.findByTestId("graph-search-result"));
  fireEvent.click(await screen.findByTestId("graph-detail-tab-neighbors"));
  await waitFor(() => expect(desktopApi.graphQuery).toHaveBeenCalledWith(expect.objectContaining({ operation: "neighbors" })));
  const callsBefore = vi.mocked(desktopApi.graphQuery).mock.calls.length;

  fireEvent.click(screen.getByRole("checkbox", { name: "governance" }));
  await waitFor(() => expect(vi.mocked(desktopApi.graphQuery).mock.calls.length).toBeGreaterThan(callsBefore));
  const lastCall = vi.mocked(desktopApi.graphQuery).mock.calls.at(-1)?.[0];
  expect(lastCall).toMatchObject({ operation: "neighbors", bounds: { layers: ["governance"] } });
});

it("truncated results disclose truncation and support load-more via the returned cursor", async () => {
  vi.mocked(desktopApi.graphQuery).mockImplementation(async (request) => {
    if (request.operation === "search" && !request.bounds?.cursor) {
      return envelope({ query: "work", matches: [{ node: workFact, rank: "prefix", matched_field: "label" }] }, { truncated: true, next_cursor: "cursor-1" });
    }
    if (request.operation === "search" && request.bounds?.cursor === "cursor-1") {
      return envelope({ query: "work", matches: [{ node: fileFact, rank: "prefix", matched_field: "label" }] }, { truncated: false, next_cursor: null });
    }
    throw new Error("unexpected search call");
  });
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "work" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  await screen.findByTestId("graph-search-load-more");
  expect(screen.getAllByTestId("graph-search-result")).toHaveLength(1);
  fireEvent.click(screen.getByTestId("graph-search-load-more"));
  await waitFor(() => expect(screen.getAllByTestId("graph-search-result")).toHaveLength(2));
});

it("impact/tests/governance use their canonical operations, never a recomputed React equivalent", async () => {
  vi.mocked(desktopApi.graphQuery).mockImplementation(async (request) => {
    if (request.operation === "search") return envelope({ query: "src/lib.rs", matches: [{ node: fileFact, rank: "exact", matched_field: "repository_relative_path" }] });
    if (request.operation === "context") return envelope({ identity: fileFact, containment: [], direct_relations: [], related_nodes: [] });
    if (request.operation === "impact") return envelope({ target: fileFact, impact_semantics: "structural_only", dependents: [workFact], test_targets: [], build_package_runtime_surfaces: [], applicable_governance: [] });
    if (request.operation === "tests") return envelope({ node: fileFact, test_targets: [], relations: [] });
    if (request.operation === "governance") return envelope({ node: fileFact, relations: [], facts: [] });
    throw new Error(`unexpected operation ${request.operation}`);
  });
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "src/lib.rs" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  fireEvent.click(await screen.findByTestId("graph-search-result"));

  fireEvent.click(await screen.findByTestId("graph-detail-tab-impact"));
  const impact = await screen.findByTestId("graph-detail-impact");
  expect(impact).toHaveTextContent("structural_only");
  await waitFor(() => expect(desktopApi.graphQuery).toHaveBeenCalledWith(expect.objectContaining({ operation: "impact" })));

  fireEvent.click(screen.getByTestId("graph-detail-tab-tests"));
  await screen.findByTestId("graph-detail-tests");
  await waitFor(() => expect(desktopApi.graphQuery).toHaveBeenCalledWith(expect.objectContaining({ operation: "tests" })));

  fireEvent.click(screen.getByTestId("graph-detail-tab-governance"));
  const governance = await screen.findByTestId("graph-detail-governance");
  expect(governance).toHaveTextContent("do not confer approval");
  await waitFor(() => expect(desktopApi.graphQuery).toHaveBeenCalledWith(expect.objectContaining({ operation: "governance" })));
});

it("source navigation for a work-item fact calls the typed navigation callback", async () => {
  vi.mocked(desktopApi.graphQuery).mockImplementation(async (request) => {
    if (request.operation === "search") return envelope({ query: "work:100", matches: [{ node: workFact, rank: "exact", matched_field: "stable_id" }] });
    if (request.operation === "context") return envelope({ identity: workFact, containment: [], direct_relations: [], related_nodes: [] });
    throw new Error(`unexpected operation ${request.operation}`);
  });
  const onNavigateToRecord = vi.fn();
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={onNavigateToRecord} />);
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "work:100" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  fireEvent.click(await screen.findByTestId("graph-search-result"));
  fireEvent.click(await screen.findByRole("button", { name: "Open record" }));
  expect(onNavigateToRecord).toHaveBeenCalledWith("work_item", "100");
});

it("verify is read-only: it calls graph_verify, never graph_build", async () => {
  vi.mocked(desktopApi.graphVerify).mockResolvedValue(freshStatus);
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  fireEvent.click(await screen.findByTestId("graph-verify-button"));
  await waitFor(() => expect(desktopApi.graphVerify).toHaveBeenCalledTimes(1));
  expect(desktopApi.graphBuild).not.toHaveBeenCalled();
});

it("rebuild requires an explicit confirmation step before graph_build is called", async () => {
  vi.mocked(desktopApi.graphBuild).mockResolvedValue({} as never);
  render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  const buildButton = await screen.findByTestId("graph-build-button");
  fireEvent.click(buildButton);
  expect(desktopApi.graphBuild).not.toHaveBeenCalled();
  await screen.findByTestId("graph-build-confirm");
  fireEvent.click(screen.getByTestId("graph-build-confirm-yes"));
  await waitFor(() => expect(desktopApi.graphBuild).toHaveBeenCalledTimes(1));
});

it("essential controls (search, select, verify) work identically in compact layout with no hover/right-click dependency", async () => {
  vi.mocked(desktopApi.graphQuery).mockImplementation(async (request) => {
    if (request.operation === "search") return envelope({ query: "src/lib.rs", matches: [{ node: fileFact, rank: "exact", matched_field: "repository_relative_path" }] });
    if (request.operation === "context") return envelope({ identity: fileFact, containment: [], direct_relations: [], related_nodes: [] });
    throw new Error(`unexpected operation ${request.operation}`);
  });
  vi.mocked(desktopApi.graphVerify).mockResolvedValue(freshStatus);
  render(<GraphOperatorMap compact onNavigateToRecord={vi.fn()} changeSignal={0} />);
  // Every essential interaction below is a plain button/input click --
  // no `mouseOver`/`mouseEnter`/`contextMenu` event is ever fired here,
  // and none is required for the compact layout to work.
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "src/lib.rs" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  fireEvent.click(await screen.findByTestId("graph-search-result"));
  await screen.findByTestId("graph-detail-identity");
  fireEvent.click(screen.getByTestId("graph-verify-button"));
  await waitFor(() => expect(desktopApi.graphVerify).toHaveBeenCalledTimes(1));
});

it("a repository-changed generation bump invalidates the selection and re-resolves it", async () => {
  let resolveCalls = 0;
  vi.mocked(desktopApi.graphQuery).mockImplementation(async (request) => {
    if (request.operation === "search") return envelope({ query: "src/lib.rs", matches: [{ node: fileFact, rank: "exact", matched_field: "repository_relative_path" }] });
    if (request.operation === "context") return envelope({ identity: fileFact, containment: [], direct_relations: [], related_nodes: [] });
    if (request.operation === "resolve") {
      resolveCalls += 1;
      return envelope({ outcome: "not_found" });
    }
    throw new Error(`unexpected operation ${request.operation}`);
  });
  const { rerender } = render(<GraphOperatorMap compact={false} changeSignal={0} onNavigateToRecord={vi.fn()} />);
  fireEvent.change(screen.getByTestId("graph-search-input"), { target: { value: "src/lib.rs" } });
  fireEvent.click(screen.getByTestId("graph-search-submit"));
  fireEvent.click(await screen.findByTestId("graph-search-result"));
  await screen.findByTestId("graph-detail-identity");

  rerender(<GraphOperatorMap compact={false} changeSignal={1} onNavigateToRecord={vi.fn()} />);
  await waitFor(() => expect(resolveCalls).toBe(1));
  await screen.findByTestId("graph-selection-stale");
});
