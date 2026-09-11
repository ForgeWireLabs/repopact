import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { onBackButtonPress } from "@tauri-apps/api/app";
import type {
  AnalysisView,
  DecisionSummaryView,
  EvidenceSummaryView,
  GraphView,
  MutationApplyView,
  MutationIntent,
  MutationPlanView,
  RecordDetailView,
  RepositoryChangedEvent,
  RepositoryOverview,
  ValidationView,
  WorkItemDetailView,
  WorkItemSummaryView,
} from "./generated/types";
import { LIFECYCLE_STATUSES } from "./generated/types";
import { desktopApi, type DesktopFailure } from "./lib/api";

/**
 * WI060 AND-011: the Android system-Back unwind order, factored out as a
 * pure function so it is testable without Tauri's Android-only
 * `onBackButtonPress` plugin event. Priority order: an open detail view
 * (Work/Decisions/Evidence) closes first and restores the underlying list
 * state (its query/tab/pager were never touched, so they persist
 * automatically); then an open compact navigation drawer; only when neither
 * is open does RepoPact have no internal mobile navigation state left to
 * unwind, and the caller should fall through to the platform's normal
 * Back/exit behavior.
 */
export type BackAction = "close-detail" | "close-nav" | "exit";

export function resolveBackAction(state: { detail: unknown; navOpen: boolean }): BackAction {
  if (state.detail) return "close-detail";
  if (state.navOpen) return "close-nav";
  return "exit";
}

export type PrimaryTab =
  | "dashboard"
  | "work"
  | "decisions"
  | "evidence"
  | "graph"
  | "validation"
  | "analysis"
  | "settings";

const primaryTabs: Array<{ id: PrimaryTab; label: string }> = [
  { id: "dashboard", label: "Dashboard" },
  { id: "work", label: "Work" },
  { id: "decisions", label: "Decisions" },
  { id: "evidence", label: "Evidence" },
  { id: "graph", label: "Graph" },
  { id: "validation", label: "Validation" },
  { id: "analysis", label: "Analysis" },
  { id: "settings", label: "Settings" },
];

export type WorkTab = "proposed" | "active" | "deferred" | "completed";
type DashboardTab = "overview" | "attention" | "session";
type DecisionTab = "current" | "proposed" | "deferred" | "history";
type EvidenceTab = "recent" | "passed" | "attention" | "all";
type GraphTab = "dependencies" | "evidence" | "governance" | "all";
type ValidationTab = "errors" | "warnings" | "info" | "all";
type AnalysisTab = "constraints" | "suggestions" | "facts";
type SettingsTab = "appearance" | "session" | "boundaries";
type SectionTab = DashboardTab | WorkTab | DecisionTab | EvidenceTab | GraphTab | ValidationTab | AnalysisTab | SettingsTab;

type DetailState =
  | { kind: "work"; id: string }
  | { kind: "decision"; id: string }
  | { kind: "evidence"; id: string }
  | null;

const DEFAULT_SECTION_TABS: Record<PrimaryTab, SectionTab> = {
  dashboard: "overview",
  work: "active",
  decisions: "current",
  evidence: "recent",
  graph: "all",
  validation: "all",
  analysis: "constraints",
  settings: "appearance",
};

const EMPTY_PAGES: Record<PrimaryTab, number> = {
  dashboard: 0,
  work: 0,
  decisions: 0,
  evidence: 0,
  graph: 0,
  validation: 0,
  analysis: 0,
  settings: 0,
};

function failureMessage(error: unknown): string {
  const failure = error as Partial<DesktopFailure>;
  return failure.message ?? (error instanceof Error ? error.message : "The desktop operation failed.");
}

function useCompactLayout(): boolean {
  const [compact, setCompact] = useState(() =>
    typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia("(max-width: 900px)").matches
      : false,
  );

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    const media = window.matchMedia("(max-width: 900px)");
    const update = () => setCompact(media.matches);
    update();
    media.addEventListener?.("change", update);
    return () => media.removeEventListener?.("change", update);
  }, []);

  return compact;
}

export interface SectionTabOption<T extends string> {
  id: T;
  label: string;
  count?: number;
}

export function SectionTabs<T extends string>({
  tabs,
  value,
  onChange,
  label,
  panelId,
}: {
  tabs: ReadonlyArray<SectionTabOption<T>>;
  value: T;
  onChange: (value: T) => void;
  label: string;
  panelId: string;
}) {
  const tablistRef = useRef<HTMLDivElement>(null);

  const moveFocus = (nextIndex: number) => {
    const buttons = tablistRef.current?.querySelectorAll<HTMLButtonElement>('[role="tab"]');
    if (!buttons?.length) return;
    const index = (nextIndex + buttons.length) % buttons.length;
    buttons[index]?.focus();
    onChange(tabs[index].id);
  };

  return (
    <div className="section-tabs" ref={tablistRef} role="tablist" aria-label={label}>
      {tabs.map((tab, index) => (
        <button
          key={tab.id}
          className={tab.id === value ? "section-tab selected" : "section-tab"}
          type="button"
          role="tab"
          aria-selected={tab.id === value}
          aria-controls={panelId}
          id={`${panelId}-tab-${tab.id}`}
          tabIndex={tab.id === value ? 0 : -1}
          onClick={() => onChange(tab.id)}
          onKeyDown={(event) => {
            if (event.key === "ArrowRight" || event.key === "ArrowDown") {
              event.preventDefault();
              moveFocus(index === tabs.length - 1 ? 0 : index + 1);
            } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
              event.preventDefault();
              moveFocus(index === 0 ? tabs.length - 1 : index - 1);
            } else if (event.key === "Home") {
              event.preventDefault();
              moveFocus(0);
            } else if (event.key === "End") {
              event.preventDefault();
              moveFocus(tabs.length - 1);
            }
          }}
        >
          <span>{tab.label}</span>
          {tab.count !== undefined && <span className="tab-count" aria-label={`${tab.count} items`}>{tab.count}</span>}
        </button>
      ))}
    </div>
  );
}

export function LocalPager({
  page,
  pageSize,
  total,
  compact,
  onPageChange,
}: {
  page: number;
  pageSize: number;
  total: number;
  compact: boolean;
  onPageChange: (page: number) => void;
}) {
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const safePage = total === 0 ? 0 : Math.min(page, pageCount - 1);

  useEffect(() => {
    if (page !== safePage) onPageChange(safePage);
  }, [onPageChange, page, safePage]);

  const start = total === 0 ? 0 : safePage * pageSize + 1;
  const end = total === 0 ? 0 : Math.min(total, (safePage + 1) * pageSize);
  return (
    <nav className="pager" aria-label="Local collection pagination" data-compact={compact}>
      <span className="pager-range">{start}–{end} of {total}</span>
      <div className="button-row">
        <button type="button" className="secondary-button" onClick={() => onPageChange(Math.max(0, safePage - 1))} disabled={safePage === 0}>Previous</button>
        <button type="button" className="secondary-button" onClick={() => onPageChange(Math.min(pageCount - 1, safePage + 1))} disabled={safePage >= pageCount - 1}>Next</button>
      </div>
    </nav>
  );
}

function tabPanelId(primary: PrimaryTab): string {
  return `${primary}-section-panel`;
}

function today() {
  return new Date().toISOString().slice(0, 10);
}

function workCounts(items: WorkItemSummaryView[]): Record<WorkTab, number> {
  return {
    proposed: items.filter((item) => item.status === "proposed").length,
    active: items.filter((item) => item.status === "active" || item.status === "blocked").length,
    deferred: items.filter((item) => item.status === "deferred").length,
    completed: items.filter((item) => item.status === "completed").length,
  };
}

function preferredWorkTab(items: WorkItemSummaryView[]): WorkTab {
  const counts = workCounts(items);
  if (counts.active > 0) return "active";
  return (["proposed", "deferred", "completed"] as WorkTab[]).find((tab) => counts[tab] > 0) ?? "proposed";
}

function decisionCategory(status: string | null): DecisionTab {
  switch (status?.toLowerCase()) {
    case "accepted": return "current";
    case "proposed": return "proposed";
    case "deferred": return "deferred";
    default: return "history";
  }
}

function evidenceCategory(result: string | null): "passed" | "attention" {
  return result?.toLowerCase() === "passed" ? "passed" : "attention";
}

function defaultValidationTab(validation: ValidationView): ValidationTab {
  if (validation.diagnostics.some((item) => item.severity === "error")) return "errors";
  if (validation.diagnostics.some((item) => item.severity === "warning")) return "warnings";
  if (validation.diagnostics.some((item) => item.severity === "info")) return "info";
  return "all";
}

function pageSlice<T>(items: T[], page: number, pageSize: number): T[] {
  return items.slice(page * pageSize, page * pageSize + pageSize);
}

function App() {
  const compact = useCompactLayout();
  const [tab, setTab] = useState<PrimaryTab>("dashboard");
  const [overview, setOverview] = useState<RepositoryOverview | null>(null);
  const [workItems, setWorkItems] = useState<WorkItemSummaryView[]>([]);
  const [decisions, setDecisions] = useState<DecisionSummaryView[]>([]);
  const [evidence, setEvidence] = useState<EvidenceSummaryView[]>([]);
  const [graph, setGraph] = useState<GraphView | null>(null);
  const [validation, setValidation] = useState<ValidationView | null>(null);
  const [analysis, setAnalysis] = useState<AnalysisView | null>(null);
  const [selectedWork, setSelectedWork] = useState<WorkItemDetailView | null>(null);
  const [selectedDecision, setSelectedDecision] = useState<RecordDetailView | null>(null);
  const [selectedEvidence, setSelectedEvidence] = useState<RecordDetailView | null>(null);
  const [detail, setDetail] = useState<DetailState>(null);
  const [plan, setPlan] = useState<MutationPlanView | null>(null);
  const [lastApply, setLastApply] = useState<MutationApplyView | null>(null);
  const [sectionTabs, setSectionTabs] = useState<Record<PrimaryTab, SectionTab>>(DEFAULT_SECTION_TABS);
  const [pages, setPages] = useState<Record<PrimaryTab, number>>(EMPTY_PAGES);
  const [workQuery, setWorkQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [navOpen, setNavOpen] = useState(false);
  const [theme, setTheme] = useState<"system" | "light" | "dark">("system");

  const setSectionTab = useCallback((section: PrimaryTab, value: SectionTab) => {
    setSectionTabs((current) => ({ ...current, [section]: value }));
    setPages((current) => ({ ...current, [section]: 0 }));
  }, []);

  const setSectionPage = useCallback((section: PrimaryTab, page: number) => {
    setPages((current) => ({ ...current, [section]: page }));
  }, []);

  const loadViews = useCallback(async (knownOverview?: RepositoryOverview, resetNavigation = false) => {
    const [nextOverview, nextWork, nextDecisions, nextEvidence, nextValidation, nextGraph, nextAnalysis] = await Promise.all([
      knownOverview ? Promise.resolve(knownOverview) : desktopApi.overview(),
      desktopApi.workItems(),
      desktopApi.decisions(),
      desktopApi.evidence(),
      desktopApi.validate(),
      desktopApi.graph(),
      desktopApi.analyze(),
    ]);
    setOverview(nextOverview);
    setWorkItems(nextWork);
    setDecisions(nextDecisions);
    setEvidence(nextEvidence);
    setValidation(nextValidation);
    setGraph(nextGraph);
    setAnalysis(nextAnalysis);
    setSelectedWork(null);
    setSelectedDecision(null);
    setSelectedEvidence(null);
    setDetail(null);
    setPages(EMPTY_PAGES);
    if (resetNavigation) setWorkQuery("");
    setSectionTabs((current) => resetNavigation
      ? { ...DEFAULT_SECTION_TABS, work: preferredWorkTab(nextWork), validation: defaultValidationTab(nextValidation) }
      : current);
  }, []);

  useEffect(() => {
    let mounted = true;
    let stop: (() => void) | undefined;
    void desktopApi.listenForChanges((event: RepositoryChangedEvent) => {
      if (!mounted) return;
      setNotice(event.origin === "self_apply" ? `Applied changes refreshed (${event.changed_paths.length} path${event.changed_paths.length === 1 ? "" : "s"}).` : `Repository changed externally; refreshed ${event.changed_paths.length} path${event.changed_paths.length === 1 ? "" : "s"}.`);
      void loadViews(event.overview).catch((operationError) => setError(failureMessage(operationError)));
    }).then((unlisten) => {
      if (mounted) stop = unlisten;
      else unlisten();
    });
    return () => {
      mounted = false;
      stop?.();
    };
  }, [loadViews]);

  // WI060 AND-011: the official Android integration point. Registering this
  // listener switches Tauri's Android runtime from its default "WebView
  // history back, else exit" behavior to fully delegating Back to RepoPact,
  // so it is registered ONLY while there is RepoPact navigation state to
  // unwind (an open detail view or the compact drawer). When
  // resolveBackAction says "exit" — no internal state left — this effect
  // deliberately does not register a listener at all, so Android's own
  // default Back/exit behavior (which already finishes the activity
  // correctly) runs unmodified rather than RepoPact trying to reimplement
  // activity-finish semantics itself.
  useEffect(() => {
    if (resolveBackAction({ detail, navOpen }) === "exit") {
      return;
    }
    let unregister: (() => void) | undefined;
    let cancelled = false;
    void onBackButtonPress(() => {
      switch (resolveBackAction({ detail, navOpen })) {
        case "close-detail":
          setDetail(null);
          break;
        case "close-nav":
          setNavOpen(false);
          break;
        case "exit":
          break;
      }
    }).then((listener) => {
      if (cancelled) {
        void listener.unregister();
        return;
      }
      unregister = () => void listener.unregister();
    }).catch(() => {
      // Not running inside a Tauri Android webview (desktop, browser dev
      // server, or the test environment): there is no Back button to own.
    });
    return () => {
      cancelled = true;
      unregister?.();
    };
  }, [detail, navOpen]);

  const selectRepository = async () => {
    setBusy(true);
    setError("");
    try {
      const selected = await desktopApi.selectRepository();
      if (selected) await loadViews(selected, true);
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
      await loadViews(nextOverview);
      setNotice("Repository snapshot refreshed.");
    } catch (operationError) {
      setError(failureMessage(operationError));
    } finally {
      setBusy(false);
    }
  };

  const openDetail = (next: DetailState) => {
    setDetail(next);
  };

  const openWorkItem = async (id: string) => {
    setError("");
    if (selectedWork?.work_item.id === id) {
      openDetail({ kind: "work", id });
      return;
    }
    try {
      setSelectedWork(await desktopApi.workItem(id));
      openDetail({ kind: "work", id });
    } catch (operationError) {
      setError(failureMessage(operationError));
    }
  };

  const openDecision = async (item: DecisionSummaryView) => {
    setError("");
    if (selectedDecision?.reference.id === item.reference.id) {
      openDetail({ kind: "decision", id: item.reference.id });
      return;
    }
    try {
      setSelectedDecision(await desktopApi.decision(item.reference.id));
      openDetail({ kind: "decision", id: item.reference.id });
    } catch (operationError) {
      setError(failureMessage(operationError));
    }
  };

  const openEvidence = async (item: EvidenceSummaryView) => {
    setError("");
    if (selectedEvidence?.reference.id === item.reference.id) {
      openDetail({ kind: "evidence", id: item.reference.id });
      return;
    }
    try {
      setSelectedEvidence(await desktopApi.evidenceRecord(item.reference.id));
      openDetail({ kind: "evidence", id: item.reference.id });
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
      await loadViews();
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

  const navigate = (nextTab: PrimaryTab) => {
    setTab(nextTab);
    setDetail(null);
    setNavOpen(false);
  };

  const openNavigation = () => {
    setNavOpen(true);
  };

  return (
    <div className="app-shell" data-theme={theme} data-layout={compact ? "compact" : "wide"}>
      <header className="topbar">
        <div className="topbar-brand">
          <button className="compact-nav-toggle" type="button" aria-label="Open workbench navigation" aria-expanded={navOpen} onClick={openNavigation}>☰</button>
          <div><p className="eyebrow">FORGEWIRELABS / GOVERNANCE</p><h1>RepoPact Workbench</h1></div>
        </div>
        <div className="topbar-actions">
          <button className="secondary-button" onClick={selectRepository} disabled={busy}>{overview ? "Switch repository" : "Select repository"}</button>
          {overview && <button className="secondary-button" onClick={refresh} disabled={busy}>Refresh</button>}
          <label className="theme-picker">Theme<select value={theme} onChange={(event) => setTheme(event.target.value as typeof theme)} aria-label="Theme"><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label>
        </div>
      </header>

      {error && <div className="alert error" role="alert"><strong>Action needed.</strong> {error}<button onClick={() => setError("")} aria-label="Dismiss error">Dismiss</button></div>}
      {notice && <div className="alert notice" role="status">{notice}<button onClick={() => setNotice("")} aria-label="Dismiss notice">Dismiss</button></div>}

      <div className="workspace-layout">
        <nav className={navOpen ? "side-nav open" : "side-nav"} aria-label="Workbench sections">
          <div className="compact-nav-heading"><strong>Sections</strong><button className="quiet-button" type="button" onClick={() => setNavOpen(false)} aria-label="Close workbench navigation">×</button></div>
          {primaryTabs.map((item) => <button key={item.id} className={tab === item.id ? "nav-item active" : "nav-item"} aria-current={tab === item.id ? "page" : undefined} onClick={() => navigate(item.id)}>{item.label}</button>)}
          <div className="nav-footer"><span className="status-dot" aria-hidden="true" /> Native session boundary<br /><small>{overview ? `Generation ${overview.generation}` : "No repository selected"}</small></div>
        </nav>
        {navOpen && <button className="nav-scrim" type="button" aria-label="Close navigation" onClick={() => setNavOpen(false)} />}
        <main className="main-content" tabIndex={-1}>
          {!overview ? <EmptyRepository onSelect={selectRepository} busy={busy} /> : <>
            <div className="page-heading"><div><p className="eyebrow">ACTIVE REPOSITORY</p><h2>{primaryTabs.find((item) => item.id === tab)?.label}</h2><p className="muted path-text">{overview.identity.root}{overview.identity.linked_worktree ? " · linked Git worktree" : ""}</p></div><span className={overview.validation.valid ? "health-pill healthy" : "health-pill unhealthy"}>{overview.validation.valid ? "Validated" : "Needs attention"}</span></div>
            {tab === "dashboard" && <Dashboard overview={overview} value={sectionTabs.dashboard as DashboardTab} onChange={(value) => setSectionTab("dashboard", value)} onOpen={navigate} />}
            {tab === "work" && <WorkPage items={workItems} query={workQuery} setQuery={(value) => { setWorkQuery(value); setSectionPage("work", 0); }} selected={selectedWork} value={sectionTabs.work as WorkTab} onChange={(value) => setSectionTab("work", value)} page={pages.work} onPageChange={(value) => setSectionPage("work", value)} compact={compact} detail={detail?.kind === "work" ? detail : null} onOpen={openWorkItem} onBack={() => setDetail(null)} onPlan={submitPlan} />}
            {tab === "decisions" && <DecisionsPage records={decisions} value={sectionTabs.decisions as DecisionTab} onChange={(value) => setSectionTab("decisions", value)} page={pages.decisions} onPageChange={(value) => setSectionPage("decisions", value)} compact={compact} detail={detail?.kind === "decision" ? detail : null} record={selectedDecision} onOpen={openDecision} onBack={() => setDetail(null)} />}
            {tab === "evidence" && <EvidencePage records={evidence} value={sectionTabs.evidence as EvidenceTab} onChange={(value) => setSectionTab("evidence", value)} page={pages.evidence} onPageChange={(value) => setSectionPage("evidence", value)} compact={compact} detail={detail?.kind === "evidence" ? detail : null} record={selectedEvidence} onOpen={openEvidence} onBack={() => setDetail(null)} />}
            {tab === "graph" && <GraphPage graph={graph} value={sectionTabs.graph as GraphTab} onChange={(value) => setSectionTab("graph", value)} page={pages.graph} onPageChange={(value) => setSectionPage("graph", value)} compact={compact} />}
            {tab === "validation" && <ValidationPage validation={validation} value={sectionTabs.validation as ValidationTab} onChange={(value) => setSectionTab("validation", value)} page={pages.validation} onPageChange={(value) => setSectionPage("validation", value)} compact={compact} />}
            {tab === "analysis" && <AnalysisPage analysis={analysis} value={sectionTabs.analysis as AnalysisTab} onChange={(value) => setSectionTab("analysis", value)} page={pages.analysis} onPageChange={(value) => setSectionPage("analysis", value)} compact={compact} />}
            {tab === "settings" && <SettingsPage overview={overview} value={sectionTabs.settings as SettingsTab} onChange={(value) => setSectionTab("settings", value)} theme={theme} setTheme={setTheme} />}
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

function Dashboard({ overview, value, onChange, onOpen }: { overview: RepositoryOverview; value: DashboardTab; onChange: (value: DashboardTab) => void; onOpen: (tab: PrimaryTab) => void }) {
  const tabs: SectionTabOption<DashboardTab>[] = [{ id: "overview", label: "Overview" }, { id: "attention", label: "Attention", count: overview.validation.error_count + overview.validation.warning_count }, { id: "session", label: "Session" }];
  return <section className="page-stack"><SectionTabs tabs={tabs} value={value} onChange={onChange} label="Dashboard views" panelId={tabPanelId("dashboard")} /><div id={tabPanelId("dashboard")} className="tab-panel" role="tabpanel" aria-labelledby={`${tabPanelId("dashboard")}-tab-${value}`} aria-label={`${value} dashboard view`}>
    {value === "overview" && <div className="dashboard-grid"><section className="hero-card"><div><p className="eyebrow">SESSION HEALTH</p><h3>{overview.validation.valid ? "The repository is in a governable state." : "Validation needs your attention."}</h3><p className="muted">{overview.validation.error_count} errors · {overview.validation.warning_count} warnings · snapshot {overview.snapshot_token.slice(0, 12)}…</p></div><button className="secondary-button" onClick={() => onOpen("validation")}>View validation</button></section><div className="metric-grid">{[["Work items", overview.work_item_count, "work"], ["Evidence runs", overview.evidence_count, "evidence"], ["Decisions", overview.decision_count, "decisions"], ["Graph edges", overview.graph_edge_count, "graph"]].map(([label, count, target]) => <button className="metric-card" key={label} onClick={() => onOpen(target as PrimaryTab)}><span>{label}</span><strong>{count}</strong><small>Open section →</small></button>)}</div></div>}
    {value === "attention" && <div className="attention-grid"><section className="panel compact-card"><p className="eyebrow">VALIDATION</p><h3>{overview.validation.error_count + overview.validation.warning_count} items need attention</h3><p className="muted">Open severity-focused diagnostics to inspect canonical remediation guidance.</p><button className="secondary-button" onClick={() => onOpen("validation")}>Open validation</button></section><section className="panel compact-card"><p className="eyebrow">BLOCKED WORK</p><h3>Blocked items stay in Active</h3><p className="muted">The Work section keeps blocked records visible under Active with their canonical status.</p><button className="secondary-button" onClick={() => onOpen("work")}>Open Active work</button></section></div>}
    {value === "session" && <SessionCard overview={overview} />}
  </div></section>;
}

function SessionCard({ overview }: { overview: RepositoryOverview }) {
  return <section className="panel session-card"><div><p className="eyebrow">SESSION</p><h3>{overview.identity.linked_worktree ? "Linked Git worktree" : "Repository identity locked"}</h3><p className="muted breakable">{overview.identity.root}</p></div><dl className="definition-grid"><div><dt>Generation</dt><dd>{overview.generation}</dd></div><div><dt>Snapshot</dt><dd>{overview.snapshot_token.slice(0, 16)}…</dd></div><div><dt>Watcher</dt><dd>{overview.watcher.state}</dd></div><div><dt>Debounce</dt><dd>{overview.watcher.debounce_ms}ms</dd></div></dl></section>;
}

function WorkPage({ items, query, setQuery, selected, value, onChange, page, onPageChange, compact, detail, onOpen, onBack, onPlan }: { items: WorkItemSummaryView[]; query: string; setQuery: (value: string) => void; selected: WorkItemDetailView | null; value: WorkTab; onChange: (value: WorkTab) => void; page: number; onPageChange: (page: number) => void; compact: boolean; detail: { id: string } | null; onOpen: (id: string) => void; onBack: () => void; onPlan: (intent: MutationIntent) => void }) {
  const [mode, setMode] = useState<"browse" | "create" | "edit" | "transition">("browse");
  const counts = workCounts(items);
  const tabs: SectionTabOption<WorkTab>[] = [{ id: "proposed", label: "Proposed", count: counts.proposed }, { id: "active", label: "Active", count: counts.active }, { id: "deferred", label: "Deferred", count: counts.deferred }, { id: "completed", label: "Complete", count: counts.completed }];
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return items.filter((item) => (value === "active" ? item.status === "active" || item.status === "blocked" : item.status === value) && (!needle || `${item.id} ${item.title} ${item.status}`.toLowerCase().includes(needle))).sort((left, right) => {
      if (value === "active" && left.status !== right.status) return left.status === "blocked" ? -1 : 1;
      return left.id < right.id ? -1 : left.id > right.id ? 1 : 0;
    });
  }, [items, query, value]);
  const pageSize = compact ? 7 : 11;
  const visible = pageSlice(filtered, page, pageSize);
  const focused = compact && (detail !== null || mode === "create");
  const list = <section className="panel list-panel"><div className="panel-heading"><div><p className="eyebrow">WORK LIFECYCLE</p><h3>Work items</h3></div><button className="primary-button" onClick={() => setMode("create")}>Create work item</button></div><label className="search-label">Search {value} work items<input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="ID, title, or status" /></label><div className="record-list grouped-list">{visible.map((item, index) => <WorkRow key={item.id} item={item} selected={selected?.work_item.id === item.id} showGroup={value === "active" && (index === 0 || visible[index - 1]?.status !== item.status)} onOpen={onOpen} />)}{visible.length === 0 && <p className="muted empty-inline">No work items match this view.</p>}</div><LocalPager page={page} pageSize={pageSize} total={filtered.length} compact={compact} onPageChange={onPageChange} /></section>;
  const detailPanel = <section className="panel detail-panel">{mode === "create" ? <CreateForm onCancel={() => setMode("browse")} onPlan={onPlan} /> : selected ? <WorkDetail detail={selected} mode={mode} setMode={setMode} onPlan={onPlan} /> : <div className="detail-placeholder"><p className="eyebrow">DETAIL</p><h3>Select a work item</h3><p className="muted">Choose an indexed record to inspect its typed fields, criteria, dependencies, and guarded intents.</p></div>}</section>;
  return <section className={focused ? "focused-detail" : "page-stack"}>{!focused && <SectionTabs tabs={tabs} value={value} onChange={onChange} label="Work lifecycle views" panelId="work-page-panel" />}{focused && <button className="back-button" type="button" onClick={() => { setMode("browse"); onBack(); }}>← Back to {tabs.find((tab) => tab.id === value)?.label}</button>}{focused ? detailPanel : <div id="work-page-panel" className="work-layout" role="tabpanel" aria-labelledby={`work-page-panel-tab-${value}`} aria-label={`${value} work view`}>{list}{detailPanel}</div>}</section>;
}

function WorkRow({ item, selected, showGroup, onOpen }: { item: WorkItemSummaryView; selected: boolean; showGroup: boolean; onOpen: (id: string) => void }) {
  return <>{showGroup && <h4 className="group-heading">{item.status === "blocked" ? "Blocked" : "In progress"}</h4>}<button className={selected ? "record-row selected" : item.status === "blocked" ? "record-row blocked" : "record-row"} onClick={() => onOpen(item.id)}><span className="record-marker">{item.status.slice(0, 1).toUpperCase()}</span><span><strong>{item.id} · {item.title}</strong><small>{item.status} · {item.owner_scope}</small></span><span aria-hidden="true">→</span></button></>;
}

function WorkDetail({ detail, mode, setMode, onPlan }: { detail: WorkItemDetailView; mode: "browse" | "edit" | "transition"; setMode: (mode: "browse" | "edit" | "transition") => void; onPlan: (intent: MutationIntent) => void }) {
  const [title, setTitle] = useState(detail.work_item.title);
  const [status, setStatus] = useState(detail.work_item.status);
  return <div><div className="panel-heading"><div><p className="eyebrow">{detail.work_item.id}</p><h3>{detail.work_item.title}</h3></div><div className="button-row"><button className="secondary-button" onClick={() => setMode("edit")}>Edit typed fields</button><button className="secondary-button" onClick={() => setMode("transition")}>Transition</button></div></div><div className="detail-stats"><span><small>Status</small><strong>{detail.work_item.status}</strong></span><span><small>Owner scope</small><strong>{detail.work_item.owner_scope}</strong></span><span><small>Criteria</small><strong>{detail.work_item.acceptance_criteria.length}</strong></span></div>{mode === "edit" && <form className="form-card" onSubmit={(event) => { event.preventDefault(); onPlan({ kind: "edit_work_item", payload: { id: detail.work_item.id, date: today(), changes: { title, owner_scope: null, affected_scopes: null, depends_on: null, provenance: null, acceptance_criteria: null } } }); }}><label>Title<input value={title} onChange={(event) => setTitle(event.target.value)} required /></label><p className="muted">Only typed fields are available here; the Rust plan retains authoritative file operations.</p><button className="primary-button" type="submit">Review edit plan</button></form>}{mode === "transition" && <form className="form-card" onSubmit={(event) => { event.preventDefault(); onPlan({ kind: "transition_work_item", payload: { id: detail.work_item.id, status } }); }}><label>Next status<select value={status} onChange={(event) => setStatus(event.target.value)}>{LIFECYCLE_STATUSES.map((value) => <option key={value}>{value}</option>)}</select></label><p className="muted">Transitions remain typed intents. The Rust core performs lifecycle and post-validation checks.</p><button className="primary-button" type="submit">Review transition plan</button></form>}<section className="subsection"><h4>Acceptance criteria</h4>{detail.work_item.acceptance_criteria.map((criterion) => <div className="criterion" key={criterion.id}><span>{criterion.id}</span><div><strong>{criterion.text}</strong><small>{criterion.state} · {criterion.evidence.length} evidence references</small></div></div>)}</section><section className="subsection"><h4>Relationships</h4><p className="muted">Depends on: {detail.work_item.depends_on.join(", ") || "none"} · Dependents: {detail.dependents.join(", ") || "none"}</p></section></div>;
}

function CreateForm({ onCancel, onPlan }: { onCancel: () => void; onPlan: (intent: MutationIntent) => void }) {
  const [step, setStep] = useState(0);
  const [title, setTitle] = useState("");
  const [owner, setOwner] = useState("governance");
  const [status, setStatus] = useState("proposed");
  const [criteria, setCriteria] = useState("");
  const criterionValues = criteria.split("\n").map((text) => text.trim()).filter(Boolean);
  const intent: MutationIntent = { kind: "create_work_item", payload: { title, status, date: today(), owner_scope: owner, affected_scopes: [], depends_on: [], provenance: "concrete", acceptance_criteria: criterionValues.map((text, index) => ({ id: `AC-${String(index + 1).padStart(2, "0")}`, text, state: "pending", evidence: [], provenance: "concrete" })) } };
  return <div><div className="panel-heading"><div><p className="eyebrow">GUIDED CREATE · STEP {step + 1} OF 3</p><h3>New work item</h3></div><button className="quiet-button" onClick={onCancel}>Cancel</button></div>{step === 0 && <div className="form-card"><label>Title<input autoFocus value={title} onChange={(event) => setTitle(event.target.value)} placeholder="What needs to be governed?" /></label><label>Owner scope<input value={owner} onChange={(event) => setOwner(event.target.value)} /></label><label>Initial status<select value={status} onChange={(event) => setStatus(event.target.value)}>{LIFECYCLE_STATUSES.map((value) => <option key={value}>{value}</option>)}</select></label><button className="primary-button" disabled={!title.trim()} onClick={() => setStep(1)}>Continue to guidance</button></div>}{step === 1 && <div className="form-card"><div className="guidance-card"><p className="eyebrow">EPHEMERAL GUIDANCE</p><h4>What will prove completion?</h4><p>Capture concise acceptance criteria now. Evidence references remain part of existing criteria semantics.</p></div><label>Acceptance criteria, one per line<textarea value={criteria} onChange={(event) => setCriteria(event.target.value)} rows={6} placeholder="A reviewer can verify ..." /></label><div className="button-row"><button className="secondary-button" onClick={() => setStep(0)}>Back</button><button className="primary-button" onClick={() => setStep(2)}>Review details</button></div></div>}{step === 2 && <div className="form-card"><div className="review-grid"><span>Title<strong>{title}</strong></span><span>Owner<strong>{owner}</strong></span><span>Status<strong>{status}</strong></span><span>Criteria<strong>{criterionValues.length}</strong></span></div><p className="muted">The next step creates a Rust-owned content-addressed plan. No repository write happens until approval.</p><div className="button-row"><button className="secondary-button" onClick={() => setStep(1)}>Back</button><button className="primary-button" onClick={() => onPlan(intent)}>Create review plan</button></div></div>}</div>;
}

export function PlanDialog({ plan, onApply, onDiscard, busy }: { plan: MutationPlanView; onApply: () => void; onDiscard: () => void; busy: boolean }) {
  return <div className="modal-backdrop" role="presentation"><section className="plan-dialog" role="dialog" aria-modal="true" aria-labelledby="plan-title"><div className="panel-heading"><div><p className="eyebrow">RUST-OWNED PLAN</p><h2 id="plan-title">Review proposed mutation</h2></div><button className="quiet-button" onClick={onDiscard}>Close</button></div><div className="handle-banner"><span>Opaque plan handle</span><code>{plan.plan_handle}</code><small>Token {plan.plan_token.slice(0, 16)}… · session-bound</small></div><pre className="preview-box">{plan.preview || "No preview was generated."}</pre>{plan.diagnostics.map((diagnostic) => <div className={diagnostic.blocking ? "diagnostic blocking" : "diagnostic"} key={`${diagnostic.code}-${diagnostic.message}`}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span></div>)}<div className="impact-list"><h3>Generated impacts</h3>{plan.generated_impacts.length === 0 ? <p className="muted">No generated dashboard impact.</p> : plan.generated_impacts.map((impact) => <div key={impact.path}><strong>{impact.path}</strong><small>{impact.reason}</small></div>)}</div><div className="button-row"><button className="secondary-button" onClick={onDiscard}>Discard plan</button><button className="primary-button" onClick={onApply} disabled={!plan.applicable || busy}>Apply approved plan</button></div>{!plan.applicable && <p className="error-text">This plan is blocked by Rust core diagnostics and cannot be applied.</p>}</section></div>;
}

function DecisionsPage({ records, value, onChange, page, onPageChange, compact, detail, record, onOpen, onBack }: { records: DecisionSummaryView[]; value: DecisionTab; onChange: (value: DecisionTab) => void; page: number; onPageChange: (page: number) => void; compact: boolean; detail: { id: string } | null; record: RecordDetailView | null; onOpen: (record: DecisionSummaryView) => void; onBack: () => void }) {
  const counts = { current: records.filter((item) => decisionCategory(item.status) === "current").length, proposed: records.filter((item) => decisionCategory(item.status) === "proposed").length, deferred: records.filter((item) => decisionCategory(item.status) === "deferred").length, history: records.filter((item) => decisionCategory(item.status) === "history").length };
  const tabs: SectionTabOption<DecisionTab>[] = [{ id: "current", label: "Current", count: counts.current }, { id: "proposed", label: "Proposed", count: counts.proposed }, { id: "deferred", label: "Deferred", count: counts.deferred }, { id: "history", label: "History", count: counts.history }];
  const sorted = records.filter((item) => decisionCategory(item.status) === value).sort((left, right) => (right.date ?? "").localeCompare(left.date ?? "") || left.reference.id.localeCompare(right.reference.id));
  const pageSize = compact ? 7 : 11;
  const focused = compact && detail !== null;
  const list = <section className="panel list-panel"><div className="panel-heading"><div><p className="eyebrow">SNAPSHOT SUMMARIES</p><h3>Decisions</h3></div></div><p className="muted">Canonical decision metadata is projected from the current Rust snapshot.</p><div className="record-list">{pageSlice(sorted, page, pageSize).map((item) => <button className="record-row" key={item.reference.id} onClick={() => onOpen(item)}><span className="record-marker">D</span><span><strong>{item.title ?? item.reference.id}</strong><small>{item.status ?? "unknown"} · {item.date ?? "undated"}</small></span><span aria-hidden="true">→</span></button>)}{sorted.length === 0 && <p className="muted empty-inline">No decisions in this view.</p>}</div><LocalPager page={page} pageSize={pageSize} total={sorted.length} compact={compact} onPageChange={onPageChange} /></section>;
  const detailPanel = <section className="panel detail-panel">{record ? <RecordDetail detail={record} label="Decision" /> : <div className="detail-placeholder"><h3>Select a decision</h3><p className="muted">Choose a snapshot summary to inspect its source-backed record.</p></div>}</section>;
  return <section className={focused ? "focused-detail" : "page-stack"}>{!focused && <SectionTabs tabs={tabs} value={value} onChange={onChange} label="Decision views" panelId="decisions-page-panel" />}{focused && <button className="back-button" type="button" onClick={onBack}>← Back to {tabs.find((tab) => tab.id === value)?.label}</button>}{focused ? detailPanel : <div id="decisions-page-panel" className="records-layout" role="tabpanel" aria-labelledby={`decisions-page-panel-tab-${value}`} aria-label={`${value} decisions view`}>{list}{detailPanel}</div>}</section>;
}

function EvidencePage({ records, value, onChange, page, onPageChange, compact, detail, record, onOpen, onBack }: { records: EvidenceSummaryView[]; value: EvidenceTab; onChange: (value: EvidenceTab) => void; page: number; onPageChange: (page: number) => void; compact: boolean; detail: { id: string } | null; record: RecordDetailView | null; onOpen: (record: EvidenceSummaryView) => void; onBack: () => void }) {
  const ordered = [...records].sort((left, right) => (right.timestamp ?? "").localeCompare(left.timestamp ?? "") || left.reference.id.localeCompare(right.reference.id));
  const recent = ordered.slice(0, 20);
  const categories: Record<EvidenceTab, EvidenceSummaryView[]> = { recent, passed: ordered.filter((item) => evidenceCategory(item.result) === "passed"), attention: ordered.filter((item) => evidenceCategory(item.result) === "attention"), all: ordered };
  const tabs: SectionTabOption<EvidenceTab>[] = [{ id: "recent", label: "Recent", count: recent.length }, { id: "passed", label: "Passed", count: categories.passed.length }, { id: "attention", label: "Attention", count: categories.attention.length }, { id: "all", label: "All", count: records.length }];
  const selected = categories[value];
  const pageSize = compact ? 7 : 11;
  const focused = compact && detail !== null;
  const list = <section className="panel list-panel"><div className="panel-heading"><div><p className="eyebrow">SNAPSHOT SUMMARIES</p><h3>Evidence</h3></div></div><p className="muted">Results, timestamps, and associated work are loaded with the immutable snapshot.</p><div className="record-list">{pageSlice(selected, page, pageSize).map((item) => <button className="record-row" key={item.reference.id} onClick={() => onOpen(item)}><span className="record-marker">{item.result === "passed" ? "✓" : "!"}</span><span><strong>{item.reference.id}</strong><small>{item.result ?? "unknown"} · {item.timestamp ?? "undated"} · {item.work_item ?? "unlinked"}</small></span><span aria-hidden="true">→</span></button>)}{selected.length === 0 && <p className="muted empty-inline">No evidence in this view.</p>}</div><LocalPager page={page} pageSize={pageSize} total={selected.length} compact={compact} onPageChange={onPageChange} /></section>;
  const detailPanel = <section className="panel detail-panel">{record ? <RecordDetail detail={record} label="Evidence run" /> : <div className="detail-placeholder"><h3>Select an evidence run</h3><p className="muted">Choose a snapshot summary to inspect its source-backed record.</p></div>}</section>;
  return <section className={focused ? "focused-detail" : "page-stack"}>{!focused && <SectionTabs tabs={tabs} value={value} onChange={onChange} label="Evidence views" panelId="evidence-page-panel" />}{focused && <button className="back-button" type="button" onClick={onBack}>← Back to {tabs.find((tab) => tab.id === value)?.label}</button>}{focused ? detailPanel : <div id="evidence-page-panel" className="records-layout" role="tabpanel" aria-labelledby={`evidence-page-panel-tab-${value}`} aria-label={`${value} evidence view`}>{list}{detailPanel}</div>}</section>;
}

function RecordDetail({ detail, label }: { detail: RecordDetailView; label: string }) {
  return <div><p className="eyebrow">{label}</p><h3>{detail.reference.id}</h3><p className="muted breakable">{detail.reference.path}</p><pre className="record-content">{detail.value ? JSON.stringify(detail.value, null, 2) : detail.text ?? "Record is not readable."}</pre></div>;
}

function GraphPage({ graph, value, onChange, page, onPageChange, compact }: { graph: GraphView | null; value: GraphTab; onChange: (value: GraphTab) => void; page: number; onPageChange: (page: number) => void; compact: boolean }) {
  const edges = graph?.edges ?? [];
  const categories: Record<GraphTab, typeof edges> = { dependencies: edges.filter((edge) => edge.kind === "depends_on" || edge.kind === "reverse_dependency"), evidence: edges.filter((edge) => edge.kind === "supported_by" || edge.kind === "supports_work_item"), governance: edges.filter((edge) => !["depends_on", "reverse_dependency", "supported_by", "supports_work_item"].includes(edge.kind)), all: edges };
  const tabs: SectionTabOption<GraphTab>[] = [{ id: "dependencies", label: "Dependencies", count: categories.dependencies.length }, { id: "evidence", label: "Evidence", count: categories.evidence.length }, { id: "governance", label: "Governance", count: categories.governance.length }, { id: "all", label: "All", count: edges.length }];
  const selected = categories[value];
  const pageSize = compact ? 6 : 10;
  const visible = pageSlice(selected, page, pageSize);
  return <section className="page-stack"><SectionTabs tabs={tabs} value={value} onChange={onChange} label="Graph relationship views" panelId="graph-page-panel" /><section id="graph-page-panel" className="panel" role="tabpanel" aria-labelledby={`graph-page-panel-tab-${value}`} aria-label={`${value} graph view`}><div className="panel-heading"><div><p className="eyebrow">RELATIONSHIP MODEL</p><h3>Repository graph</h3></div><span className="tag">{compact ? "Stacked accessible view" : "Table alternative"}</span></div>{!graph ? <p className="muted">Loading graph…</p> : <><div className="table-wrap wide-only"><table><caption className="sr-only">Repository relationship edges</caption><thead><tr><th>From</th><th>Relationship</th><th>To</th><th>Source</th></tr></thead><tbody>{visible.map((edge, index) => <tr key={`${edge.from}-${edge.to}-${index}`}><td>{edge.from}</td><td>{edge.kind}</td><td>{edge.to}</td><td>{edge.source.path}</td></tr>)}</tbody></table></div><div className="graph-cards compact-only">{visible.map((edge, index) => <article className="relationship-card" key={`${edge.from}-${edge.to}-${index}`}><strong>{edge.kind}</strong><dl><div><dt>From</dt><dd>{edge.from}</dd></div><div><dt>To</dt><dd>{edge.to}</dd></div><div><dt>Source</dt><dd>{edge.source.path}</dd></div></dl></article>)}</div>{selected.length === 0 && <p className="muted empty-inline">No relationships in this view.</p>}<LocalPager page={page} pageSize={pageSize} total={selected.length} compact={compact} onPageChange={onPageChange} /></>}</section></section>;
}

function ValidationPage({ validation, value, onChange, page, onPageChange, compact }: { validation: ValidationView | null; value: ValidationTab; onChange: (value: ValidationTab) => void; page: number; onPageChange: (page: number) => void; compact: boolean }) {
  if (!validation) return <section className="panel"><p className="muted">Loading validation…</p></section>;
  if (validation.diagnostics.length === 0) return <section className="panel success-card"><p className="eyebrow">CANONICAL VALIDATION</p><h3>All clear</h3><p>No validation diagnostics were reported for this snapshot.</p></section>;
  const categories: Record<ValidationTab, typeof validation.diagnostics> = { errors: validation.diagnostics.filter((item) => item.severity === "error"), warnings: validation.diagnostics.filter((item) => item.severity === "warning"), info: validation.diagnostics.filter((item) => item.severity === "info"), all: validation.diagnostics };
  const tabs: SectionTabOption<ValidationTab>[] = [{ id: "errors", label: "Errors", count: categories.errors.length }, { id: "warnings", label: "Warnings", count: categories.warnings.length }, { id: "info", label: "Info", count: categories.info.length }, { id: "all", label: "All", count: categories.all.length }];
  const selected = categories[value];
  const pageSize = compact ? 7 : 11;
  return <section className="page-stack"><SectionTabs tabs={tabs} value={value} onChange={onChange} label="Validation severity views" panelId="validation-page-panel" /><section id="validation-page-panel" className="panel" role="tabpanel" aria-labelledby={`validation-page-panel-tab-${value}`} aria-label={`${value} validation view`}><div className="panel-heading"><div><p className="eyebrow">CANONICAL VALIDATION</p><h3>Repository diagnostics</h3></div><span className={validation.valid ? "health-pill healthy" : "health-pill unhealthy"}>{validation.valid ? "Valid" : "Invalid"}</span></div><div className="diagnostic-list">{pageSlice(selected, page, pageSize).map((item, index) => <div className={`diagnostic ${item.severity}`} key={`${item.code}-${index}`}><strong>{item.code}</strong><span>{item.message}</span><small>{item.path ?? item.record ?? "repository"}</small></div>)}</div><LocalPager page={page} pageSize={pageSize} total={selected.length} compact={compact} onPageChange={onPageChange} /></section></section>;
}

function AnalysisPage({ analysis, value, onChange, page, onPageChange, compact }: { analysis: AnalysisView | null; value: AnalysisTab; onChange: (value: AnalysisTab) => void; page: number; onPageChange: (page: number) => void; compact: boolean }) {
  if (!analysis) return <section className="panel"><p className="muted">Loading analysis…</p></section>;
  const categories: Record<AnalysisTab, typeof analysis.findings> = { constraints: analysis.findings.filter((item) => item.classification === "constraint"), suggestions: analysis.findings.filter((item) => item.classification === "suggestion"), facts: analysis.findings.filter((item) => item.classification === "fact") };
  const tabs: SectionTabOption<AnalysisTab>[] = [{ id: "constraints", label: "Constraints", count: categories.constraints.length }, { id: "suggestions", label: "Suggestions", count: categories.suggestions.length }, { id: "facts", label: "Facts", count: categories.facts.length }];
  const selected = categories[value];
  const pageSize = compact ? 7 : 11;
  return <section className="page-stack"><SectionTabs tabs={tabs} value={value} onChange={onChange} label="Analysis finding views" panelId="analysis-page-panel" /><section id="analysis-page-panel" className="panel" role="tabpanel" aria-labelledby={`analysis-page-panel-tab-${value}`} aria-label={`${value} analysis view`}><div className="panel-heading"><div><p className="eyebrow">EXPLAINABLE GUIDANCE</p><h3>Analysis findings</h3></div></div>{selected.length === 0 ? <p className="muted">No findings in this view.</p> : <div className="finding-list">{pageSlice(selected, page, pageSize).map((finding, index) => <article className="finding" key={`${finding.code}-${index}`}><span className="tag">{finding.classification}</span><h4>{finding.code}</h4><p>{finding.message}</p>{finding.remediation && <small>Next step: {finding.remediation}</small>}{finding.basis.length > 0 && <small className="basis">Basis: {finding.basis.map((item) => item.path).join(", ")}</small>}</article>)}</div>}<LocalPager page={page} pageSize={pageSize} total={selected.length} compact={compact} onPageChange={onPageChange} /></section></section>;
}

function SettingsPage({ overview, value, onChange, theme, setTheme }: { overview: RepositoryOverview; value: SettingsTab; onChange: (value: SettingsTab) => void; theme: "system" | "light" | "dark"; setTheme: (theme: "system" | "light" | "dark") => void }) {
  const tabs: SectionTabOption<SettingsTab>[] = [{ id: "appearance", label: "Appearance" }, { id: "session", label: "Session" }, { id: "boundaries", label: "Boundaries" }];
  return <section className="page-stack"><SectionTabs tabs={tabs} value={value} onChange={onChange} label="Settings views" panelId="settings-page-panel" /><section id="settings-page-panel" className="panel" role="tabpanel" aria-labelledby={`settings-page-panel-tab-${value}`} aria-label={`${value} settings view`}><div className="panel-heading"><div><p className="eyebrow">LOCAL WORKBENCH</p><h3>Settings</h3></div><span className="tag">Presentation preferences</span></div>{value === "appearance" && <div className="settings-grid"><div><h4>Theme</h4><p className="muted">Choose how the workbench presents the current session.</p><label>Color mode<select value={theme} onChange={(event) => setTheme(event.target.value as typeof theme)}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label></div><div><h4>Adaptive layout</h4><p className="muted">The same information architecture adapts to available width, touch input, safe areas, and orientation.</p></div></div>}{value === "session" && <SessionCard overview={overview} />}{value === "boundaries" && <div className="boundary-card"><h4>Native authority boundary</h4><p className="muted">Repository selection, indexed reads, validation, watching, and approved typed mutations remain in the Rust desktop adapter. This window has no generic filesystem or shell commands. Mutation plans remain opaque, session-bound Rust memory.</p></div>}</section></section>;
}

export default App;
