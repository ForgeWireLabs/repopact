import { useEffect, useMemo, useState } from "react";
import type {
  AnalysisView,
  GraphView,
  MutationApplyView,
  MutationIntent,
  MutationPlanView,
  RecordDetailView,
  RecordSummaryView,
  RepositoryChangedEvent,
  RepositoryOverview,
  ValidationView,
  WorkItemDetailView,
  WorkItemSummaryView,
} from "./generated/types";
import { LIFECYCLE_STATUSES } from "./generated/types";
import { desktopApi, type DesktopFailure } from "./lib/api";

type Tab = "dashboard" | "work" | "decisions" | "evidence" | "graph" | "validation" | "analysis" | "settings";
const tabs: Array<{ id: Tab; label: string }> = [
  { id: "dashboard", label: "Dashboard" },
  { id: "work", label: "Work" },
  { id: "decisions", label: "Decisions" },
  { id: "evidence", label: "Evidence" },
  { id: "graph", label: "Graph" },
  { id: "validation", label: "Validation" },
  { id: "analysis", label: "Analysis" },
  { id: "settings", label: "Settings" },
];

function failureMessage(error: unknown): string {
  const failure = error as Partial<DesktopFailure>;
  return failure.message ?? (error instanceof Error ? error.message : "The desktop operation failed.");
}

function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [overview, setOverview] = useState<RepositoryOverview | null>(null);
  const [workItems, setWorkItems] = useState<WorkItemSummaryView[]>([]);
  const [selectedWork, setSelectedWork] = useState<WorkItemDetailView | null>(null);
  const [decisions, setDecisions] = useState<RecordSummaryView[]>([]);
  const [evidence, setEvidence] = useState<RecordSummaryView[]>([]);
  const [record, setRecord] = useState<RecordDetailView | null>(null);
  const [graph, setGraph] = useState<GraphView | null>(null);
  const [validation, setValidation] = useState<ValidationView | null>(null);
  const [analysis, setAnalysis] = useState<AnalysisView | null>(null);
  const [plan, setPlan] = useState<MutationPlanView | null>(null);
  const [lastApply, setLastApply] = useState<MutationApplyView | null>(null);
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [theme, setTheme] = useState<"system" | "light" | "dark">("system");

  const loadRepository = async () => {
    setBusy(true);
    setError("");
    try {
      const [nextOverview, nextWork, nextDecisions, nextEvidence] = await Promise.all([
        desktopApi.overview(),
        desktopApi.workItems(),
        desktopApi.decisions(),
        desktopApi.evidence(),
      ]);
      setOverview(nextOverview);
      setWorkItems(nextWork);
      setDecisions(nextDecisions);
      setEvidence(nextEvidence);
      setSelectedWork(null);
      setRecord(null);
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    let mounted = true;
    let stop: (() => void) | undefined;
    void desktopApi.listenForChanges((event: RepositoryChangedEvent) => {
      if (!mounted) return;
      setOverview(event.overview);
      setNotice(
        event.origin === "self_apply"
          ? `Applied changes refreshed (${event.changed_paths.length} path${event.changed_paths.length === 1 ? "" : "s"}).`
          : `Repository changed externally; refreshed ${event.changed_paths.length} path${event.changed_paths.length === 1 ? "" : "s"}.`,
      );
      void desktopApi.workItems().then(setWorkItems).catch((operationError) => setError(failureMessage(operationError)));
    }).then((unlisten) => {
      if (mounted) stop = unlisten;
      else unlisten();
    });
    return () => {
      mounted = false;
      stop?.();
    };
  }, []);

  useEffect(() => {
    if (!overview || tab !== "validation") return;
    void desktopApi.validate().then(setValidation).catch((operationError) => setError(failureMessage(operationError)));
  }, [overview, tab]);

  useEffect(() => {
    if (!overview || tab !== "graph") return;
    void desktopApi.graph().then(setGraph).catch((operationError) => setError(failureMessage(operationError)));
  }, [overview, tab]);

  useEffect(() => {
    if (!overview || tab !== "analysis") return;
    void desktopApi
      .analyze(selectedWork ? { candidate_id: selectedWork.work_item.id } : undefined)
      .then(setAnalysis)
      .catch((operationError) => setError(failureMessage(operationError)));
  }, [overview, selectedWork, tab]);

  const visibleWork = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return workItems;
    return workItems.filter((item) => `${item.id} ${item.title} ${item.status}`.toLowerCase().includes(needle));
  }, [query, workItems]);

  const selectRepository = async () => {
    setBusy(true);
    setError("");
    try {
      const selected = await desktopApi.selectRepository();
      if (selected) await loadRepository();
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  };

  const refresh = async () => {
    setBusy(true);
    setError("");
    try {
      const nextOverview = await desktopApi.refresh();
      setOverview(nextOverview);
      setWorkItems(await desktopApi.workItems());
      setNotice("Repository snapshot refreshed.");
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  };

  const openWorkItem = async (id: string) => {
    setError("");
    try {
      setSelectedWork(await desktopApi.workItem(id));
      setTab("work");
    } catch (operationError) {
      setError(failureMessage(operationError));
    }
  };

  const openRecord = async (kind: "decision" | "evidence", item: RecordSummaryView) => {
    setError("");
    try {
      setRecord(kind === "decision" ? await desktopApi.decision(item.reference.id) : await desktopApi.evidenceRecord(item.reference.id));
    } catch (operationError) {
      setError(failureMessage(operationError));
    }
  };

  const submitPlan = async (intent: MutationIntent) => {
    setBusy(true);
    setError("");
    setLastApply(null);
    try {
      setPlan(await desktopApi.plan(intent));
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  };

  const applyPlan = async () => {
    if (!plan || !overview) return;
    setBusy(true);
    setError("");
    try {
      const result = await desktopApi.apply(plan.session_id, plan.plan_handle);
      setLastApply(result);
      setPlan(null);
      await loadRepository();
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  };

  const discardPlan = async () => {
    if (!plan) return;
    try {
      await desktopApi.discard(plan.session_id, plan.plan_handle);
      setPlan(null);
    } catch (operationError) {
      setError(failureMessage(operationError));
    }
  };

  return (
    <div className="app-shell" data-theme={theme}>
      <header className="topbar">
        <div>
          <p className="eyebrow">FORGEWIRELABS / GOVERNANCE</p>
          <h1>RepoPact Workbench</h1>
        </div>
        <div className="topbar-actions">
          <button className="secondary-button" onClick={selectRepository} disabled={busy}>
            {overview ? "Switch repository" : "Select repository"}
          </button>
          {overview && <button className="secondary-button" onClick={refresh} disabled={busy}>Refresh</button>}
          <label className="theme-picker">Theme
            <select value={theme} onChange={(event) => setTheme(event.target.value as typeof theme)} aria-label="Theme">
              <option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option>
            </select>
          </label>
        </div>
      </header>

      {error && <div className="alert error" role="alert"><strong>Action needed.</strong> {error}<button onClick={() => setError("")} aria-label="Dismiss error">Dismiss</button></div>}
      {notice && <div className="alert notice" role="status">{notice}<button onClick={() => setNotice("")} aria-label="Dismiss notice">Dismiss</button></div>}

      <div className="workspace-layout">
        <nav className="side-nav" aria-label="Workbench sections">
          {tabs.map((item) => <button key={item.id} className={tab === item.id ? "nav-item active" : "nav-item"} aria-current={tab === item.id ? "page" : undefined} onClick={() => setTab(item.id)}>{item.label}</button>)}
          <div className="nav-footer"><span className="status-dot" aria-hidden="true" /> Native session boundary<br /><small>{overview ? `Generation ${overview.generation}` : "No repository selected"}</small></div>
        </nav>
        <main className="main-content" tabIndex={-1}>
          {!overview ? <EmptyRepository onSelect={selectRepository} busy={busy} /> : <>
            <div className="page-heading"><div><p className="eyebrow">ACTIVE REPOSITORY</p><h2>{tabs.find((item) => item.id === tab)?.label}</h2><p className="muted path-text">{overview.identity.root}{overview.identity.linked_worktree ? " · linked Git worktree" : ""}</p></div><span className={overview.validation.valid ? "health-pill healthy" : "health-pill unhealthy"}>{overview.validation.valid ? "Validated" : "Needs attention"}</span></div>
            {tab === "dashboard" && <Dashboard overview={overview} onOpen={(nextTab) => setTab(nextTab)} />}
            {tab === "work" && <WorkPage items={visibleWork} query={query} setQuery={setQuery} selected={selectedWork} onOpen={openWorkItem} onPlan={submitPlan} />}
            {tab === "decisions" && <RecordsPage title="Decisions" description="Browse indexed decision records. Editing decisions is intentionally outside this workbench." records={decisions} record={record} onOpen={(item) => void openRecord("decision", item)} />}
            {tab === "evidence" && <RecordsPage title="Evidence" description="Browse indexed evidence runs and their results." records={evidence} record={record} onOpen={(item) => void openRecord("evidence", item)} />}
            {tab === "graph" && <GraphPage graph={graph} />}
            {tab === "validation" && <ValidationPage validation={validation} />}
            {tab === "analysis" && <AnalysisPage analysis={analysis} />}
            {tab === "settings" && <SettingsPage overview={overview} />}
          </>}
        </main>
      </div>

      {plan && <PlanDialog plan={plan} onApply={applyPlan} onDiscard={discardPlan} busy={busy} />}
      {lastApply && <div className="toast" role="status">{lastApply.success ? "Plan applied and post-validation completed." : lastApply.stale ? "Plan was stale; no mutation was applied." : "Plan failed and was rolled back."}</div>}
    </div>
  );
}

function EmptyRepository({ onSelect, busy }: { onSelect: () => void; busy: boolean }) {
  return <section className="empty-state"><div className="empty-icon" aria-hidden="true">◎</div><p className="eyebrow">START A SESSION</p><h2>Select a repository to begin</h2><p>RepoPact keeps repository reads, validation, graph analysis, and approved typed changes behind a Rust-owned desktop session.</p><button className="primary-button" onClick={onSelect} disabled={busy}>Choose repository</button></section>;
}

function Dashboard({ overview, onOpen }: { overview: RepositoryOverview; onOpen: (tab: Tab) => void }) {
  const cards = [["Work items", overview.work_item_count, "work"], ["Evidence runs", overview.evidence_count, "evidence"], ["Decisions", overview.decision_count, "decisions"], ["Graph edges", overview.graph_edge_count, "graph"]] as const;
  return <div className="dashboard-grid"><section className="hero-card"><div><p className="eyebrow">SESSION HEALTH</p><h3>{overview.validation.valid ? "The repository is in a governable state." : "Validation needs your attention."}</h3><p className="muted">{overview.validation.error_count} errors · {overview.validation.warning_count} warnings · snapshot {overview.snapshot_token.slice(0, 12)}…</p></div><button className="secondary-button" onClick={() => onOpen("validation")}>View validation</button></section><div className="metric-grid">{cards.map(([label, value, target]) => <button className="metric-card" key={label} onClick={() => onOpen(target as Tab)}><span>{label}</span><strong>{value}</strong><small>Open section →</small></button>)}</div><section className="panel split-panel"><div><p className="eyebrow">WATCHER</p><h3>{overview.watcher.state === "running" ? "Live refresh is active" : "Live refresh unavailable"}</h3><p className="muted">Recursive native watcher · {overview.watcher.debounce_ms}ms debounce</p></div><div className="callout"><span className="status-dot" aria-hidden="true" />{overview.identity.linked_worktree ? "Linked worktree identity detected" : "Repository identity locked"}<small>Session generation {overview.generation}</small></div></section></div>;
}

function WorkPage({ items, query, setQuery, selected, onOpen, onPlan }: { items: WorkItemSummaryView[]; query: string; setQuery: (value: string) => void; selected: WorkItemDetailView | null; onOpen: (id: string) => void; onPlan: (intent: MutationIntent) => void }) {
  const [mode, setMode] = useState<"browse" | "create" | "edit" | "transition">("browse");
  return <div className="work-layout"><section className="panel list-panel"><div className="panel-heading"><div><p className="eyebrow">INDEXED RECORDS</p><h3>Work items</h3></div><button className="primary-button" onClick={() => setMode("create")}>Create work item</button></div><label className="search-label">Search work items<input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="ID, title, or status" /></label><div className="record-list">{items.map((item) => <button key={item.id} className={selected?.work_item.id === item.id ? "record-row selected" : "record-row"} onClick={() => { setMode("browse"); onOpen(item.id); }}><span className="record-marker">{item.status.slice(0, 1).toUpperCase()}</span><span><strong>{item.id} · {item.title}</strong><small>{item.status} · {item.owner_scope}</small></span><span aria-hidden="true">→</span></button>)}{items.length === 0 && <p className="muted empty-inline">No work items match this search.</p>}</div></section><section className="panel detail-panel">{mode === "create" ? <CreateForm onCancel={() => setMode("browse")} onPlan={onPlan} /> : selected ? <WorkDetail detail={selected} mode={mode} setMode={setMode} onPlan={onPlan} /> : <div className="detail-placeholder"><p className="eyebrow">DETAIL</p><h3>Select a work item</h3><p className="muted">Choose an indexed record to inspect its typed fields, criteria, dependencies, and available guarded intents.</p></div>}</section></div>;
}

function WorkDetail({ detail, mode, setMode, onPlan }: { detail: WorkItemDetailView; mode: "browse" | "edit" | "transition"; setMode: (mode: "browse" | "edit" | "transition") => void; onPlan: (intent: MutationIntent) => void }) {
  const [title, setTitle] = useState(detail.work_item.title);
  const [status, setStatus] = useState(detail.work_item.status);
  return <div><div className="panel-heading"><div><p className="eyebrow">{detail.work_item.id}</p><h3>{detail.work_item.title}</h3></div><div className="button-row"><button className="secondary-button" onClick={() => setMode("edit")}>Edit typed fields</button><button className="secondary-button" onClick={() => setMode("transition")}>Transition</button></div></div><div className="detail-stats"><span><small>Status</small><strong>{detail.work_item.status}</strong></span><span><small>Owner scope</small><strong>{detail.work_item.owner_scope}</strong></span><span><small>Criteria</small><strong>{detail.work_item.acceptance_criteria.length}</strong></span></div>{mode === "edit" && <form className="form-card" onSubmit={(event) => { event.preventDefault(); onPlan({ kind: "edit_work_item", payload: { id: detail.work_item.id, date: today(), changes: { title, owner_scope: null, affected_scopes: null, depends_on: null, provenance: null, acceptance_criteria: null } } }); }}><label>Title<input value={title} onChange={(event) => setTitle(event.target.value)} required /></label><p className="muted">Only the typed work-item fields are available here; the Rust plan retains the authoritative file operations.</p><button className="primary-button" type="submit">Review edit plan</button></form>}{mode === "transition" && <form className="form-card" onSubmit={(event) => { event.preventDefault(); onPlan({ kind: "transition_work_item", payload: { id: detail.work_item.id, status } }); }}><label>Next status<select value={status} onChange={(event) => setStatus(event.target.value)}>{LIFECYCLE_STATUSES.map((value) => <option key={value}>{value}</option>)}</select></label><p className="muted">Transitions are typed intents. The Rust core performs the existing lifecycle and post-validation checks.</p><button className="primary-button" type="submit">Review transition plan</button></form>}<section className="subsection"><h4>Acceptance criteria</h4>{detail.work_item.acceptance_criteria.map((criterion) => <div className="criterion" key={criterion.id}><span>{criterion.id}</span><div><strong>{criterion.text}</strong><small>{criterion.state} · {criterion.evidence.length} evidence references</small></div></div>)}</section><section className="subsection"><h4>Relationships</h4><p className="muted">Depends on: {detail.work_item.depends_on.join(", ") || "none"} · Dependents: {detail.dependents.join(", ") || "none"}</p></section></div>;
}

function CreateForm({ onCancel, onPlan }: { onCancel: () => void; onPlan: (intent: MutationIntent) => void }) {
  const [step, setStep] = useState(0);
  const [title, setTitle] = useState("");
  const [owner, setOwner] = useState("governance");
  const [status, setStatus] = useState("proposed");
  const [criteria, setCriteria] = useState("");
  const criterionValues = criteria.split("\n").map((text) => text.trim()).filter(Boolean);
  const intent: MutationIntent = { kind: "create_work_item", payload: { title, status, date: today(), owner_scope: owner, affected_scopes: [], depends_on: [], provenance: "concrete", acceptance_criteria: criterionValues.map((text, index) => ({ id: `AC-${String(index + 1).padStart(2, "0")}`, text, state: "pending", evidence: [], provenance: "concrete" })) } };
  return <div><div className="panel-heading"><div><p className="eyebrow">GUIDED CREATE · STEP {step + 1} OF 3</p><h3>New work item</h3></div><button className="quiet-button" onClick={onCancel}>Cancel</button></div>{step === 0 && <div className="form-card"><label>Title<input autoFocus value={title} onChange={(event) => setTitle(event.target.value)} placeholder="What needs to be governed?" /></label><label>Owner scope<input value={owner} onChange={(event) => setOwner(event.target.value)} /></label><label>Initial status<select value={status} onChange={(event) => setStatus(event.target.value)}>{LIFECYCLE_STATUSES.map((value) => <option key={value}>{value}</option>)}</select></label><button className="primary-button" disabled={!title.trim()} onClick={() => setStep(1)}>Continue to guidance</button></div>}{step === 1 && <div className="form-card"><div className="guidance-card"><p className="eyebrow">EPHEMERAL GUIDANCE</p><h4>What will prove completion?</h4><p>Capture concise acceptance criteria now. Evidence references remain part of existing criteria semantics and are not created by this flow.</p></div><label>Acceptance criteria, one per line<textarea value={criteria} onChange={(event) => setCriteria(event.target.value)} rows={6} placeholder="A reviewer can verify ..." /></label><div className="button-row"><button className="secondary-button" onClick={() => setStep(0)}>Back</button><button className="primary-button" onClick={() => setStep(2)}>Review details</button></div></div>}{step === 2 && <div className="form-card"><div className="review-grid"><span>Title<strong>{title}</strong></span><span>Owner<strong>{owner}</strong></span><span>Status<strong>{status}</strong></span><span>Criteria<strong>{criterionValues.length}</strong></span></div><p className="muted">The next step creates a Rust-owned content-addressed plan. No repository write happens until you approve that plan.</p><div className="button-row"><button className="secondary-button" onClick={() => setStep(1)}>Back</button><button className="primary-button" onClick={() => onPlan(intent)}>Create review plan</button></div></div>}</div>;
}

export function PlanDialog({ plan, onApply, onDiscard, busy }: { plan: MutationPlanView; onApply: () => void; onDiscard: () => void; busy: boolean }) {
  return <div className="modal-backdrop" role="presentation"><section className="plan-dialog" role="dialog" aria-modal="true" aria-labelledby="plan-title"><div className="panel-heading"><div><p className="eyebrow">RUST-OWNED PLAN</p><h2 id="plan-title">Review proposed mutation</h2></div><button className="quiet-button" onClick={onDiscard}>Close</button></div><div className="handle-banner"><span>Opaque plan handle</span><code>{plan.plan_handle}</code><small>Token {plan.plan_token.slice(0, 16)}… · session-bound</small></div><pre className="preview-box">{plan.preview || "No preview was generated."}</pre>{plan.diagnostics.map((diagnostic) => <div className={diagnostic.blocking ? "diagnostic blocking" : "diagnostic"} key={`${diagnostic.code}-${diagnostic.message}`}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span></div>)}<div className="impact-list"><h3>Generated impacts</h3>{plan.generated_impacts.length === 0 ? <p className="muted">No generated dashboard impact.</p> : plan.generated_impacts.map((impact) => <div key={impact.path}><strong>{impact.path}</strong><small>{impact.reason}</small></div>)}</div><div className="button-row"><button className="secondary-button" onClick={onDiscard}>Discard plan</button><button className="primary-button" onClick={onApply} disabled={!plan.applicable || busy}>Apply approved plan</button></div>{!plan.applicable && <p className="error-text">This plan is blocked by the Rust core diagnostics and cannot be applied.</p>}</section></div>;
}

function RecordsPage({ title, description, records, record, onOpen }: { title: string; description: string; records: RecordSummaryView[]; record: RecordDetailView | null; onOpen: (item: RecordSummaryView) => void }) {
  return <div className="records-layout"><section className="panel list-panel"><div className="panel-heading"><div><p className="eyebrow">BROWSE ONLY</p><h3>{title}</h3></div></div><p className="muted">{description}</p><div className="record-list">{records.map((item) => <button className="record-row" key={`${item.reference.kind}-${item.reference.id}`} onClick={() => onOpen(item)}><span className="record-marker">{item.readable ? "✓" : "!"}</span><span><strong>{item.reference.id}</strong><small>{item.reference.path}</small></span><span aria-hidden="true">→</span></button>)}{records.length === 0 && <p className="muted empty-inline">No indexed records found.</p>}</div></section><section className="panel detail-panel">{record ? <><p className="eyebrow">{record.reference.kind}</p><h3>{record.reference.id}</h3><p className="muted">{record.reference.path}</p><pre className="record-content">{record.value ? JSON.stringify(record.value, null, 2) : record.text ?? "Record is not readable."}</pre></> : <div className="detail-placeholder"><h3>Select a record</h3><p className="muted">Raw access is limited to indexed record identity and reference.</p></div>}</section></div>;
}

function GraphPage({ graph }: { graph: GraphView | null }) {
  return <section className="panel"><div className="panel-heading"><div><p className="eyebrow">RELATIONSHIP MODEL</p><h3>Repository graph</h3></div><span className="tag">Table alternative</span></div>{!graph ? <p className="muted">Loading graph…</p> : <div className="table-wrap"><table><caption className="sr-only">Repository relationship edges</caption><thead><tr><th>From</th><th>Relationship</th><th>To</th><th>Source</th></tr></thead><tbody>{graph.edges.map((edge, index) => <tr key={`${edge.from}-${edge.to}-${index}`}><td>{edge.from}</td><td>{edge.kind}</td><td>{edge.to}</td><td>{edge.source.path}</td></tr>)}</tbody></table></div>}</section>;
}

function ValidationPage({ validation }: { validation: ValidationView | null }) {
  return <section className="panel"><div className="panel-heading"><div><p className="eyebrow">CANONICAL VALIDATION</p><h3>Repository diagnostics</h3></div>{validation && <span className={validation.valid ? "health-pill healthy" : "health-pill unhealthy"}>{validation.valid ? "Valid" : "Invalid"}</span>}</div>{!validation ? <p className="muted">Loading validation…</p> : validation.diagnostics.length === 0 ? <div className="success-card">No validation diagnostics were reported.</div> : <div className="diagnostic-list">{validation.diagnostics.map((item, index) => <div className={`diagnostic ${item.severity}`} key={`${item.code}-${index}`}><strong>{item.code}</strong><span>{item.message}</span><small>{item.path ?? item.record ?? "repository"}</small></div>)}</div>}</section>;
}

function AnalysisPage({ analysis }: { analysis: AnalysisView | null }) {
  return <section className="panel"><div className="panel-heading"><div><p className="eyebrow">EXPLAINABLE GUIDANCE</p><h3>Analysis findings</h3></div></div>{!analysis ? <p className="muted">Loading analysis…</p> : analysis.findings.length === 0 ? <p className="muted">No findings for this snapshot.</p> : <div className="finding-list">{analysis.findings.map((finding, index) => <article className="finding" key={`${finding.code}-${index}`}><span className="tag">{finding.classification}</span><h4>{finding.code}</h4><p>{finding.message}</p>{finding.remediation && <small>Next step: {finding.remediation}</small>}</article>)}</div>}</section>;
}

function SettingsPage({ overview }: { overview: RepositoryOverview }) {
  return <section className="panel"><div className="panel-heading"><div><p className="eyebrow">LOCAL WORKBENCH</p><h3>Settings and boundaries</h3></div><span className="tag">Presentation preferences</span></div><div className="settings-grid"><div><h4>Session</h4><p className="muted">Generation {overview.generation} · {overview.identity.linked_worktree ? "linked Git worktree" : "normal Git repository"}</p><p className="muted">Plans are held in Rust memory and expire when the session changes or a plan is consumed.</p></div><div><h4>Native authority</h4><p className="muted">Repository selection, indexed reads, validation, watching, and approved typed mutations remain in the Rust desktop adapter. This window has no generic filesystem or shell commands.</p></div></div></section>;
}

function today() { return new Date().toISOString().slice(0, 10); }

export default App;
