import { useCallback, useEffect, useRef, useState } from "react";
import type {
  FactRef,
  GraphLayer,
  GraphQueryRequest,
  GraphStatusView,
  QueryBounds,
  SearchMatch,
} from "./generated/types";
import { desktopApi, type DesktopFailure } from "./lib/api";

/**
 * ROG-027 (Decision 0052): the Workbench operator repository map.
 *
 * This component is a thin typed client over the canonical bounded query
 * system (`repopact_graph::query::GraphQueryEngine`, reached through the
 * one `graph_query` Tauri command). It never re-implements traversal,
 * search ranking, or impact/test/governance semantics in React -- every
 * fact shown here came back from a typed `QueryEnvelope<...>` the Rust
 * kernel produced. The only thing this component decides on its own is
 * how to lay those facts out for an operator.
 */

type DetailTab = "identity" | "neighbors" | "impact" | "tests" | "governance";

type Envelope<T> = {
  query_contract_version: number;
  graph_schema_version: number;
  graph_fingerprint: string;
  status: {
    basis: "durable" | "working_overlay";
    durable_freshness: "absent" | "fresh" | "stale" | "unsupported" | "corrupt";
    coverage: "complete" | "partial";
  };
  warnings: string[];
  truncated: boolean;
  returned_nodes: number;
  returned_edges: number;
  next_cursor?: string | null;
  result: T;
};

type ResolutionOutcome =
  | { outcome: "exact"; fact: FactRef }
  | { outcome: "ambiguous"; candidates: FactRef[] }
  | { outcome: "not_found" };

interface RelationFact {
  from: string;
  to: string;
  kind: string;
  layer: GraphLayer;
  role?: string | null;
  derivation: string;
  source: { kind: string; id: string; path: string };
  location?: unknown;
  provenance: "persisted" | "query_derived_inverse";
}

interface ContextResult {
  identity: FactRef;
  containment: RelationFact[];
  direct_relations: RelationFact[];
  related_nodes: FactRef[];
}

interface NeighborsResult {
  node: FactRef;
  relations: RelationFact[];
  neighbors: FactRef[];
}

interface ImpactResult {
  target: FactRef;
  impact_semantics: "structural_only";
  dependents: FactRef[];
  test_targets: FactRef[];
  build_package_runtime_surfaces: FactRef[];
  applicable_governance: FactRef[];
}

interface TestsResult {
  node: FactRef;
  test_targets: FactRef[];
  relations: RelationFact[];
}

interface GovernanceResult {
  node: FactRef;
  relations: RelationFact[];
  facts: FactRef[];
}

const LAYER_OPTIONS: GraphLayer[] = ["governance", "physical", "semantic", "build", "test", "runtime", "package"];

function failureMessage(error: unknown): string {
  const failure = error as Partial<DesktopFailure>;
  return failure.message ?? (error instanceof Error ? error.message : "The graph query failed.");
}

async function query<T>(request: GraphQueryRequest): Promise<Envelope<T>> {
  return (await desktopApi.graphQuery(request)) as unknown as Envelope<T>;
}

/**
 * The single freshness/coverage/capability label an operator sees. Every
 * distinct state named by Decision 0051/0052 renders distinguishable
 * text, never color alone.
 */
export function graphStateLabel(status: GraphStatusView | null, basis?: "durable" | "working_overlay", durableFreshness?: string, coverage?: string): string {
  if (!status) return "Loading graph state…";
  if (status.capability_state === "explicit_disabled") return "Graph disabled";
  if (status.capability_state === "enabled_missing") return "Graph enabled but missing (invalid)";
  if (status.freshness === "corrupt") return "Graph corrupt";
  if (status.freshness === "unsupported") return "Graph schema unsupported";
  const legacy = status.capability_state === "legacy_enabled" ? "legacy enabled" : "enabled";
  if (basis === "working_overlay") {
    return coverage === "partial" ? `Working overlay (${legacy}, partial coverage)` : `Working overlay (${legacy})`;
  }
  if (durableFreshness === "stale") return `Durable graph stale (${legacy})`;
  if (status.freshness === "partial") return `Durable graph partial (${legacy})`;
  if (status.freshness === "absent") return "No durable graph (legacy absent)";
  return `Durable graph fresh (${legacy})`;
}

export function GraphOperatorMap({ compact, changeSignal, onNavigateToRecord }: {
  compact: boolean;
  /** Bumped by the parent whenever a `repository-changed` event lands
   * (Decision 0047/0052): a branch checkout or watcher-driven edit can
   * replace many paths at once, so any cursor/selection tied to the
   * previous graph generation must be re-validated, never trusted as-is. */
  changeSignal: number;
  onNavigateToRecord: (kind: string, id: string) => void;
}) {
  const [status, setStatus] = useState<GraphStatusView | null>(null);
  const [statusError, setStatusError] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const [searchText, setSearchText] = useState("");
  const [searchResults, setSearchResults] = useState<SearchMatch[]>([]);
  const [searchCursor, setSearchCursor] = useState<string | null>(null);
  const [searchTruncated, setSearchTruncated] = useState(false);
  const [searchBasis, setSearchBasis] = useState<Envelope<unknown>["status"] | null>(null);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedStale, setSelectedStale] = useState(false);
  const [identity, setIdentity] = useState<ContextResult | null>(null);
  const [neighbors, setNeighbors] = useState<NeighborsResult | null>(null);
  const [neighborsEnvelope, setNeighborsEnvelope] = useState<Envelope<NeighborsResult | null> | null>(null);
  const [impact, setImpact] = useState<ImpactResult | null>(null);
  const [tests, setTests] = useState<TestsResult | null>(null);
  const [governance, setGovernance] = useState<GovernanceResult | null>(null);
  const [detailTab, setDetailTab] = useState<DetailTab>("identity");

  const [layerFilter, setLayerFilter] = useState<GraphLayer[]>([]);
  const [direction, setDirection] = useState<"outgoing" | "incoming" | "both">("both");

  const [buildConfirming, setBuildConfirming] = useState(false);
  const generationRef = useRef(changeSignal);

  const bounds = useCallback((cursor?: string | null): QueryBounds => ({
    ...(layerFilter.length > 0 ? { layers: layerFilter } : {}),
    ...(cursor ? { cursor } : {}),
  }), [layerFilter]);

  const loadStatus = useCallback(async () => {
    try {
      const next = await desktopApi.graphStatus();
      setStatus(next);
      setStatusError("");
    } catch (operationError) {
      setStatusError(failureMessage(operationError));
    }
  }, []);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  const runSearch = useCallback(async (text: string, cursor?: string) => {
    setError("");
    setBusy(true);
    try {
      const envelope = await query<{ query: string; matches: SearchMatch[] }>({
        operation: "search",
        text,
        bounds: bounds(cursor),
      });
      setSearchResults((current) => (cursor ? [...current, ...envelope.result.matches] : envelope.result.matches));
      setSearchCursor(envelope.next_cursor ?? null);
      setSearchTruncated(envelope.truncated);
      setSearchBasis(envelope.status);
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  }, [bounds]);

  const loadIdentity = useCallback(async (id: string) => {
    const envelope = await query<ContextResult | null>({ operation: "context", node_id: id, bounds: bounds() });
    if (!envelope.result) {
      setSelectedStale(true);
      setIdentity(null);
      return false;
    }
    setIdentity(envelope.result);
    setSelectedStale(false);
    return true;
  }, [bounds]);

  const loadNeighbors = useCallback(async (id: string, cursor?: string) => {
    const envelope = await query<NeighborsResult | null>({
      operation: "neighbors",
      node_id: id,
      direction,
      bounds: bounds(cursor),
    });
    setNeighborsEnvelope(envelope);
    setNeighbors((current) => {
      if (!envelope.result) return null;
      if (cursor && current) {
        return {
          node: envelope.result.node,
          relations: [...current.relations, ...envelope.result.relations],
          neighbors: [...current.neighbors, ...envelope.result.neighbors],
        };
      }
      return envelope.result;
    });
  }, [bounds, direction]);

  const loadImpact = useCallback(async (id: string) => {
    const envelope = await query<ImpactResult | null>({ operation: "impact", node_id: id, bounds: bounds() });
    setImpact(envelope.result);
  }, [bounds]);

  const loadTests = useCallback(async (id: string) => {
    const envelope = await query<TestsResult | null>({ operation: "tests", node_id: id, bounds: bounds() });
    setTests(envelope.result);
  }, [bounds]);

  const loadGovernance = useCallback(async (id: string) => {
    const envelope = await query<GovernanceResult | null>({ operation: "governance", node_id: id, bounds: bounds() });
    setGovernance(envelope.result);
  }, [bounds]);

  const selectNode = useCallback(async (id: string) => {
    setError("");
    setBusy(true);
    setSelectedId(id);
    setDetailTab("identity");
    setNeighbors(null);
    setNeighborsEnvelope(null);
    setImpact(null);
    setTests(null);
    setGovernance(null);
    try {
      await loadIdentity(id);
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  }, [loadIdentity]);

  // Decision 0052: a branch checkout or watcher-driven refresh can
  // replace many repository paths at once. Any old cursor is tied to a
  // graph generation that may no longer exist, so it is dropped, and the
  // current selection (if any) is re-resolved rather than silently
  // trusted or silently swapped for a different same-named node.
  useEffect(() => {
    if (changeSignal === generationRef.current) return;
    generationRef.current = changeSignal;
    setSearchCursor(null);
    void loadStatus();
    if (!selectedId) return;
    (async () => {
      const envelope = await query<ResolutionOutcome>({
        operation: "resolve",
        selector: { type: "node_id", value: selectedId },
      });
      if (envelope.result.outcome === "exact") {
        await selectNode(selectedId);
      } else {
        setSelectedStale(true);
        setIdentity(null);
      }
    })().catch((operationError) => setError(failureMessage(operationError)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [changeSignal]);

  useEffect(() => {
    if (!selectedId) return;
    if (detailTab === "neighbors" && !neighbors) void loadNeighbors(selectedId).catch((operationError) => setError(failureMessage(operationError)));
    if (detailTab === "impact" && !impact) void loadImpact(selectedId).catch((operationError) => setError(failureMessage(operationError)));
    if (detailTab === "tests" && !tests) void loadTests(selectedId).catch((operationError) => setError(failureMessage(operationError)));
    if (detailTab === "governance" && !governance) void loadGovernance(selectedId).catch((operationError) => setError(failureMessage(operationError)));
  }, [detailTab, selectedId, neighbors, impact, tests, governance, loadNeighbors, loadImpact, loadTests, loadGovernance]);

  // Re-fetch the currently open bounded neighbor set when a filter
  // changes -- the request itself changes, never a client-side hide of
  // an already-fetched list.
  useEffect(() => {
    if (!selectedId || detailTab !== "neighbors") return;
    setNeighbors(null);
    void loadNeighbors(selectedId).catch((operationError) => setError(failureMessage(operationError)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [layerFilter, direction]);

  const runVerify = async () => {
    setBusy(true);
    setError("");
    try {
      setStatus(await desktopApi.graphVerify());
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  };

  const runBuild = async () => {
    setBusy(true);
    setError("");
    setBuildConfirming(false);
    try {
      await desktopApi.graphBuild();
      await loadStatus();
      if (selectedId) await selectNode(selectedId);
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  };

  const buildLabel = status?.capability_state === "explicit_disabled" || status?.capability_state === "legacy_absent"
    ? "Build and enable graph"
    : "Rebuild graph";

  const identityFact: FactRef | null = identity?.identity ?? neighbors?.node ?? null;

  return (
    <section className="page-stack" data-testid="graph-operator-map">
      <div className="panel graph-status-banner" role="status" data-testid="graph-freshness-banner">
        <p className="eyebrow">REPOSITORY ORIENTATION GRAPH</p>
        <h3>{graphStateLabel(status, searchBasis?.basis, searchBasis?.durable_freshness, searchBasis?.coverage)}</h3>
        {statusError && <p className="muted">{statusError}</p>}
        {status?.freshness === "partial" && <p className="muted">Semantic coverage is partial: some supported files were not fully processed.</p>}
        <div className="button-row">
          <button type="button" className="secondary-button" onClick={runVerify} disabled={busy} data-testid="graph-verify-button">Verify</button>
          {!buildConfirming
            ? <button type="button" className="secondary-button" onClick={() => setBuildConfirming(true)} disabled={busy} data-testid="graph-build-button">{buildLabel}</button>
            : <span className="button-row" data-testid="graph-build-confirm">
                <span className="muted">{buildLabel} now? This is a durable write.</span>
                <button type="button" className="primary-button" onClick={runBuild} disabled={busy} data-testid="graph-build-confirm-yes">Confirm</button>
                <button type="button" className="quiet-button" onClick={() => setBuildConfirming(false)} disabled={busy}>Cancel</button>
              </span>}
        </div>
      </div>

      <div className={compact ? "operator-map-layout compact" : "operator-map-layout wide"}>
        <section className="panel" aria-label="Search the repository orientation graph">
          <div className="panel-heading"><div><p className="eyebrow">FIND</p><h3>Search</h3></div></div>
          <label className="search-label">Node, path, symbol, or work item
            <input
              value={searchText}
              onChange={(event) => setSearchText(event.target.value)}
              onKeyDown={(event) => { if (event.key === "Enter") void runSearch(searchText); }}
              placeholder="e.g. work:063 or src/lib.rs"
              data-testid="graph-search-input"
            />
          </label>
          <div className="button-row">
            <button type="button" className="primary-button" onClick={() => void runSearch(searchText)} disabled={busy || searchText.trim().length === 0} data-testid="graph-search-submit">Search</button>
          </div>
          <fieldset className="filter-fieldset">
            <legend>Layer filter</legend>
            {LAYER_OPTIONS.map((layer) => (
              <label key={layer} className="checkbox-label">
                <input
                  type="checkbox"
                  checked={layerFilter.includes(layer)}
                  onChange={(event) => setLayerFilter((current) => event.target.checked ? [...current, layer] : current.filter((item) => item !== layer))}
                />
                {layer}
              </label>
            ))}
          </fieldset>
          <div className="record-list" data-testid="graph-search-results">
            {searchResults.map((match) => (
              <button
                key={match.node.id}
                className={selectedId === match.node.id ? "record-row selected" : "record-row"}
                onClick={() => void selectNode(match.node.id)}
                data-testid="graph-search-result"
              >
                <span className="record-marker">{match.rank === "exact" ? "=" : match.rank === "exact_normalized" ? "≈" : match.rank === "prefix" ? "»" : "…"}</span>
                <span><strong>{match.node.label || match.node.id}</strong><small>{match.node.kind} · {match.node.source?.path ?? match.node.id}</small></span>
              </button>
            ))}
            {searchResults.length === 0 && <p className="muted empty-inline">No search performed yet, or no matches.</p>}
          </div>
          {searchTruncated && searchCursor && (
            <button type="button" className="secondary-button" onClick={() => void runSearch(searchText, searchCursor ?? undefined)} disabled={busy} data-testid="graph-search-load-more">
              Load more results
            </button>
          )}
        </section>

        <section className="panel" aria-label="Selected node detail">
          {!selectedId && <p className="muted empty-inline">Select a search result to inspect it.</p>}
          {selectedId && selectedStale && (
            <p className="muted" data-testid="graph-selection-stale">
              This node no longer resolves in the current graph (repository state changed). Search again to find its replacement, if any.
            </p>
          )}
          {selectedId && !selectedStale && identityFact && <>
            <div className="panel-heading"><div><p className="eyebrow">{identityFact.kind}</p><h3>{identityFact.label || identityFact.id}</h3></div></div>
            <p className="muted">{identityFact.source?.path ?? identityFact.id}</p>
            {identityFact.source && (
              <div className="button-row">
                <button type="button" className="secondary-button" onClick={() => onNavigateToRecord(identityFact.source!.kind, identityFact.source!.id)}>Open record</button>
                <button type="button" className="quiet-button" onClick={() => void navigator.clipboard?.writeText(identityFact.source!.path)}>Copy path</button>
              </div>
            )}
            <div className="section-tabs" role="tablist" aria-label="Node detail views">
              {(["identity", "neighbors", "impact", "tests", "governance"] as DetailTab[]).map((tabId) => (
                <button
                  key={tabId}
                  type="button"
                  role="tab"
                  aria-selected={detailTab === tabId}
                  className={detailTab === tabId ? "section-tab selected" : "section-tab"}
                  onClick={() => setDetailTab(tabId)}
                  data-testid={`graph-detail-tab-${tabId}`}
                >
                  {tabId}
                </button>
              ))}
            </div>

            {detailTab === "identity" && identity && (
              <div data-testid="graph-detail-identity">
                <h4>Containment</h4>
                <ul className="fact-list">{identity.containment.map((rel, index) => <li key={index}>{rel.kind} → {rel.to}</li>)}</ul>
                <h4>Direct relations</h4>
                <ul className="fact-list">{identity.direct_relations.map((rel, index) => <li key={index}>{rel.kind} {rel.from === identityFact.id ? "→" : "←"} {rel.from === identityFact.id ? rel.to : rel.from}</li>)}</ul>
                {identity.direct_relations.length === 0 && identity.containment.length === 0 && <p className="muted empty-inline">No direct relations recorded.</p>}
              </div>
            )}

            {detailTab === "neighbors" && (
              <div data-testid="graph-detail-neighbors">
                <fieldset className="filter-fieldset">
                  <legend>Direction</legend>
                  {(["outgoing", "incoming", "both"] as const).map((option) => (
                    <label key={option} className="checkbox-label">
                      <input type="radio" name="neighbor-direction" checked={direction === option} onChange={() => setDirection(option)} />
                      {option}
                    </label>
                  ))}
                </fieldset>
                {neighborsEnvelope?.truncated && <p className="muted" data-testid="graph-neighbors-truncated">More relationships exist than are shown below.</p>}
                <ul className="fact-list">
                  {neighbors?.relations.map((rel, index) => (
                    <li key={index}>
                      <button type="button" className="quiet-button" onClick={() => void selectNode(rel.from === selectedId ? rel.to : rel.from)}>
                        {rel.kind} {rel.from === selectedId ? "→" : "←"} {rel.from === selectedId ? rel.to : rel.from}
                      </button>
                    </li>
                  ))}
                </ul>
                {neighborsEnvelope?.next_cursor && (
                  <button type="button" className="secondary-button" onClick={() => selectedId && void loadNeighbors(selectedId, neighborsEnvelope.next_cursor ?? undefined)} data-testid="graph-neighbors-load-more">
                    Load more neighbors
                  </button>
                )}
              </div>
            )}

            {detailTab === "impact" && impact && (
              <div data-testid="graph-detail-impact">
                <p className="tag">{impact.impact_semantics}</p>
                <h4>Dependents ({impact.dependents.length})</h4>
                <ul className="fact-list">{impact.dependents.map((fact) => <li key={fact.id}>{fact.label || fact.id}</li>)}</ul>
                <h4>Test targets ({impact.test_targets.length})</h4>
                <ul className="fact-list">{impact.test_targets.map((fact) => <li key={fact.id}>{fact.label || fact.id}</li>)}</ul>
              </div>
            )}

            {detailTab === "tests" && tests && (
              <div data-testid="graph-detail-tests">
                <ul className="fact-list">{tests.test_targets.map((fact) => <li key={fact.id}>{fact.label || fact.id}</li>)}</ul>
                {tests.test_targets.length === 0 && <p className="muted empty-inline">No known tests recorded for this node (this does not prove none exist).</p>}
              </div>
            )}

            {detailTab === "governance" && governance && (
              <div data-testid="graph-detail-governance">
                <p className="muted">Governance facts are informational: they do not confer approval or bypass frozen-surface authority.</p>
                <ul className="fact-list">{governance.facts.map((fact) => <li key={fact.id}>{fact.label || fact.id}</li>)}</ul>
              </div>
            )}
          </>}
        </section>
      </div>

      {error && <p className="muted" role="alert" data-testid="graph-operator-map-error">{error}</p>}
    </section>
  );
}
