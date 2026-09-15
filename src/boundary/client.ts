import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  COMMAND_BUILD_PLAN,
  COMMAND_CANCEL_SCAN,
  COMMAND_EXECUTE_PLAN,
  COMMAND_FETCH_APPLICATION_INVENTORY,
  COMMAND_FETCH_FINDINGS_PAGE,
  COMMAND_FETCH_FOLDER_AGGREGATE,
  COMMAND_FETCH_PLAN,
  COMMAND_FOUNDATION_STATUS,
  COMMAND_REVALIDATE_PLAN,
  COMMAND_START_SCAN,
  EVENT_COVERAGE_WARNING,
  EVENT_EXECUTION_OUTCOME,
  EVENT_SCAN_PROGRESS,
  EVENT_TERMINAL_COMPLETION,
  EVENT_VERIFIED_COUNT,
  type ApplicationInventory,
  type BuildPlanArgs,
  type CancelScanArgs,
  type CancelScanResult,
  type CoverageWarningPayload,
  type ExecutePlanArgs,
  type ExecutionOutcomePayload,
  type ExecutionSummary,
  type FetchFindingsPageArgs,
  type FetchFolderAggregateArgs,
  type FetchPlanArgs,
  type FindingsPage,
  type FolderAggregate,
  type FoundationStatus,
  type ReviewPlan,
  type ReviewPlanHeader,
  type RevalidatePlanArgs,
  type RevalidationResult,
  type ScanProgressPayload,
  type ScanSessionHeader,
  type StartScanArgs,
  type TerminalCompletionPayload,
  type VerifiedCountPayload,
} from "../types/bindings";

// --- Typed Read Commands ---

export async function invokeFoundationStatus(): Promise<FoundationStatus> {
  return invoke<FoundationStatus>(COMMAND_FOUNDATION_STATUS);
}

export async function invokeStartScan(
  args: StartScanArgs,
): Promise<ScanSessionHeader> {
  return invoke<ScanSessionHeader>(COMMAND_START_SCAN, { args });
}

export async function invokeCancelScan(
  args: CancelScanArgs,
): Promise<CancelScanResult> {
  return invoke<CancelScanResult>(COMMAND_CANCEL_SCAN, { args });
}

export async function invokeFetchFindingsPage(
  args: FetchFindingsPageArgs,
): Promise<FindingsPage> {
  return invoke<FindingsPage>(COMMAND_FETCH_FINDINGS_PAGE, { args });
}

export async function invokeFetchFolderAggregate(
  args: FetchFolderAggregateArgs,
): Promise<FolderAggregate> {
  return invoke<FolderAggregate>(COMMAND_FETCH_FOLDER_AGGREGATE, { args });
}

export async function invokeFetchApplicationInventory(): Promise<ApplicationInventory> {
  return invoke<ApplicationInventory>(COMMAND_FETCH_APPLICATION_INVENTORY);
}

// --- Typed Plan Commands ---

export async function invokeBuildPlan(
  args: BuildPlanArgs,
): Promise<ReviewPlanHeader> {
  return invoke<ReviewPlanHeader>(COMMAND_BUILD_PLAN, { args });
}

export async function invokeFetchPlan(
  args: FetchPlanArgs,
): Promise<ReviewPlan> {
  return invoke<ReviewPlan>(COMMAND_FETCH_PLAN, { args });
}

export async function invokeRevalidatePlan(
  args: RevalidatePlanArgs,
): Promise<RevalidationResult> {
  return invoke<RevalidationResult>(COMMAND_REVALIDATE_PLAN, { args });
}

// --- Typed Destructive Commands ---

export async function invokeExecutePlan(
  args: ExecutePlanArgs,
): Promise<ExecutionSummary> {
  return invoke<ExecutionSummary>(COMMAND_EXECUTE_PLAN, { args });
}

// --- Typed Event Listeners ---

export async function subscribeScanProgress(
  handler: (payload: ScanProgressPayload) => void,
): Promise<UnlistenFn> {
  return listen<ScanProgressPayload>(EVENT_SCAN_PROGRESS, (event) => {
    handler(event.payload);
  });
}

export async function subscribeVerifiedCount(
  handler: (payload: VerifiedCountPayload) => void,
): Promise<UnlistenFn> {
  return listen<VerifiedCountPayload>(EVENT_VERIFIED_COUNT, (event) => {
    handler(event.payload);
  });
}

export async function subscribeCoverageWarning(
  handler: (payload: CoverageWarningPayload) => void,
): Promise<UnlistenFn> {
  return listen<CoverageWarningPayload>(EVENT_COVERAGE_WARNING, (event) => {
    handler(event.payload);
  });
}

export async function subscribeExecutionOutcome(
  handler: (payload: ExecutionOutcomePayload) => void,
): Promise<UnlistenFn> {
  return listen<ExecutionOutcomePayload>(EVENT_EXECUTION_OUTCOME, (event) => {
    handler(event.payload);
  });
}

export async function subscribeTerminalCompletion(
  handler: (payload: TerminalCompletionPayload) => void,
): Promise<UnlistenFn> {
  return listen<TerminalCompletionPayload>(
    EVENT_TERMINAL_COMPLETION,
    (event) => {
      handler(event.payload);
    },
  );
}
