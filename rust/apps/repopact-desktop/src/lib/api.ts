import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AnalysisView,
  AnalyzeRequest,
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
} from "../generated/types";

export type DesktopFailure = { code: string; message: string };

export const desktopApi = {
  selectRepository: () => invoke<RepositoryOverview | null>("select_repository"),
  closeRepository: () => invoke<void>("close_repository"),
  overview: () => invoke<RepositoryOverview>("repository_overview"),
  refresh: () => invoke<RepositoryOverview>("refresh_repository"),
  validate: () => invoke<ValidationView>("validate_repository"),
  workItems: (query?: string) => invoke<WorkItemSummaryView[]>("list_work_items", { query }),
  workItem: (id: string) => invoke<WorkItemDetailView>("get_work_item", { id }),
  decisions: () => invoke<DecisionSummaryView[]>("list_decisions"),
  decision: (id: string) => invoke<RecordDetailView>("get_decision", { id }),
  evidence: () => invoke<EvidenceSummaryView[]>("list_evidence"),
  evidenceRecord: (id: string) => invoke<RecordDetailView>("get_evidence", { id }),
  graph: () => invoke<GraphView>("relationship_graph"),
  analyze: (request?: AnalyzeRequest) => invoke<AnalysisView>("analyze_work_item", { request }),
  plan: (intent: MutationIntent) => invoke<MutationPlanView>("plan_mutation", { intent }),
  apply: (sessionId: string, planHandle: string) =>
    invoke<MutationApplyView>("apply_mutation_plan", { sessionId, planHandle }),
  discard: (sessionId: string, planHandle: string) =>
    invoke<void>("discard_mutation_plan", { sessionId, planHandle }),
  listenForChanges: (handler: (event: RepositoryChangedEvent) => void): Promise<UnlistenFn> =>
    listen<RepositoryChangedEvent>("repository-changed", (event) => handler(event.payload)),
};
