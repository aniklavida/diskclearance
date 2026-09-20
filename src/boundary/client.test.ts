import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  invokeBuildPlan,
  invokeCancelScan,
  invokeExecutePlan,
  invokeFetchApplicationInventory,
  invokeFetchFindingsPage,
  invokeFetchFolderAggregate,
  invokeFetchHistoryOperations,
  invokeFetchLifetimeReclamationTotals,
  invokeFetchOperationDetail,
  invokeFetchPlan,
  invokeFoundationStatus,
  invokeRestoreItem,
  invokeRevalidatePlan,
  invokeStartScan,
  subscribeCoverageWarning,
  subscribeExecutionOutcome,
  subscribeScanProgress,
  subscribeTerminalCompletion,
  subscribeVerifiedCount,
} from "./client";
import {
  COMMAND_BUILD_PLAN,
  COMMAND_CANCEL_SCAN,
  COMMAND_EXECUTE_PLAN,
  COMMAND_FETCH_APPLICATION_INVENTORY,
  COMMAND_FETCH_FINDINGS_PAGE,
  COMMAND_FETCH_FOLDER_AGGREGATE,
  COMMAND_FETCH_HISTORY_OPERATIONS,
  COMMAND_FETCH_LIFETIME_RECLAMATION_TOTALS,
  COMMAND_FETCH_OPERATION_DETAIL,
  COMMAND_FETCH_PLAN,
  COMMAND_FOUNDATION_STATUS,
  COMMAND_RESTORE_ITEM,
  COMMAND_REVALIDATE_PLAN,
  COMMAND_START_SCAN,
  EVENT_COVERAGE_WARNING,
  EVENT_EXECUTION_OUTCOME,
  EVENT_SCAN_PROGRESS,
  EVENT_TERMINAL_COMPLETION,
  EVENT_VERIFIED_COUNT,
  type CommandError,
  type ExecutePlanArgs,
  type FoundationStatus,
} from "../types/bindings";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

describe("boundary client commands", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("dispatches foundation_status with generated FoundationStatus type", async () => {
    const mockStatus: FoundationStatus = {
      product: "DiskClearance",
      status: "Pre-implementation foundation",
      platform: "macos",
      scanningImplemented: false,
      deletionImplemented: false,
    };

    vi.mocked(invoke).mockResolvedValueOnce(mockStatus);

    const result = await invokeFoundationStatus();
    expect(invoke).toHaveBeenCalledWith(COMMAND_FOUNDATION_STATUS);
    expect(result).toEqual(mockStatus);
  });

  it("dispatches start_scan with typed arguments", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      sessionId: "s1",
      startedAtMs: 12345n,
      roots: ["/tmp"],
    });

    const result = await invokeStartScan({ roots: ["/tmp"] });
    expect(invoke).toHaveBeenCalledWith(COMMAND_START_SCAN, {
      args: { roots: ["/tmp"] },
    });
    expect(result.sessionId).toBe("s1");
  });

  it("dispatches cancel_scan with typed arguments", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      sessionId: "s1",
      cancelled: true,
    });

    const result = await invokeCancelScan({ sessionId: "s1" });
    expect(invoke).toHaveBeenCalledWith(COMMAND_CANCEL_SCAN, {
      args: { sessionId: "s1" },
    });
    expect(result.cancelled).toBe(true);
  });

  it("dispatches fetch_findings_page with pagination limit", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      sessionId: "s1",
      items: [],
      nextCursor: null,
      totalEstimated: 0n,
    });

    const result = await invokeFetchFindingsPage({
      sessionId: "s1",
      cursor: null,
      limit: 50,
    });
    expect(invoke).toHaveBeenCalledWith(COMMAND_FETCH_FINDINGS_PAGE, {
      args: { sessionId: "s1", cursor: null, limit: 50 },
    });
    expect(result.items).toHaveLength(0);
  });

  it("dispatches fetch_folder_aggregate", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      path: "/test",
      totalSizeBytes: 100n,
      fileCount: 2n,
      children: [],
    });

    const result = await invokeFetchFolderAggregate({
      sessionId: "s1",
      path: "/test",
    });
    expect(invoke).toHaveBeenCalledWith(COMMAND_FETCH_FOLDER_AGGREGATE, {
      args: { sessionId: "s1", path: "/test" },
    });
    expect(result.path).toBe("/test");
  });

  it("dispatches fetch_application_inventory", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ applications: [] });

    const result = await invokeFetchApplicationInventory();
    expect(invoke).toHaveBeenCalledWith(COMMAND_FETCH_APPLICATION_INVENTORY);
    expect(result.applications).toHaveLength(0);
  });

  it("dispatches build_plan with finding IDs", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      planId: "p1",
      sessionId: "s1",
      itemCount: 1n,
      totalBytes: 500n,
      createdAtMs: 123n,
    });

    const result = await invokeBuildPlan({
      sessionId: "s1",
      findingIds: ["f1"],
    });
    expect(invoke).toHaveBeenCalledWith(COMMAND_BUILD_PLAN, {
      args: { sessionId: "s1", findingIds: ["f1"] },
    });
    expect(result.planId).toBe("p1");
  });

  it("dispatches fetch_plan and revalidate_plan", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        planId: "p1",
        sessionId: "s1",
        items: [],
        defaultActionMode: "trash",
        createdAtMs: 123n,
      })
      .mockResolvedValueOnce({ planId: "p1", isValid: true, staleItemIds: [] });

    const plan = await invokeFetchPlan({ planId: "p1" });
    const reval = await invokeRevalidatePlan({ planId: "p1" });

    expect(invoke).toHaveBeenNthCalledWith(1, COMMAND_FETCH_PLAN, {
      args: { planId: "p1" },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, COMMAND_REVALIDATE_PLAN, {
      args: { planId: "p1" },
    });
    expect(plan.defaultActionMode).toBe("trash");
    expect(reval.isValid).toBe(true);
  });

  it("enforces execute_plan takes only planId and actionMode without path", async () => {
    const args: ExecutePlanArgs = {
      planId: "p-secure-99",
      actionMode: "trash",
    };

    vi.mocked(invoke).mockResolvedValueOnce({
      planId: "p-secure-99",
      actionMode: "trash",
      succeededItems: 1n,
      failedItems: 0n,
      bytesFreed: 1024n,
    });

    const summary = await invokeExecutePlan(args);
    expect(invoke).toHaveBeenCalledWith(COMMAND_EXECUTE_PLAN, { args });
    expect(summary.planId).toBe("p-secure-99");
  });

  it("dispatches fetch_history_operations with typed filters", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([
      {
        id: "op-1",
        planId: "p1",
        actionMode: "trash",
        createdAtMs: 100n,
        completedAtMs: 105n,
        succeededItems: 1n,
        failedItems: 0n,
        skippedItems: 0n,
        blockedItems: 0n,
        vanishedItems: 0n,
        permissionDeniedItems: 0n,
        bytesPendingTrash: 500n,
        bytesPermanentlyReclaimed: 0n,
        totalItems: 1n,
      },
    ]);

    const ops = await invokeFetchHistoryOperations({
      outcomeFilter: "succeeded",
      dateFromMs: 0n,
      dateToMs: 200n,
    });
    expect(invoke).toHaveBeenCalledWith(COMMAND_FETCH_HISTORY_OPERATIONS, {
      args: {
        outcomeFilter: "succeeded",
        dateFromMs: 0n,
        dateToMs: 200n,
      },
    });
    expect(ops).toHaveLength(1);
    expect(ops[0].id).toBe("op-1");
  });

  it("dispatches fetch_operation_detail with operationId and outcomeFilter", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      operation: {
        id: "op-1",
        planId: "p1",
        actionMode: "trash",
        createdAtMs: 100n,
        completedAtMs: 105n,
        succeededItems: 1n,
        failedItems: 0n,
        skippedItems: 0n,
        blockedItems: 0n,
        vanishedItems: 0n,
        permissionDeniedItems: 0n,
        bytesPendingTrash: 500n,
        bytesPermanentlyReclaimed: 0n,
        totalItems: 1n,
      },
      items: [],
    });

    const detail = await invokeFetchOperationDetail({
      operationId: "op-1",
      outcomeFilter: null,
    });
    expect(invoke).toHaveBeenCalledWith(COMMAND_FETCH_OPERATION_DETAIL, {
      args: {
        operationId: "op-1",
        outcomeFilter: null,
      },
    });
    expect(detail.operation.id).toBe("op-1");
  });

  it("dispatches restore_item with operationItemId and alternateDestination", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      restoreId: "res-1",
      operationItemId: "item-1",
      restoredToPath: "/restored/file.txt",
      sizeBytes: 500n,
      restoredAtMs: 200n,
    });

    const res = await invokeRestoreItem({
      operationItemId: "item-1",
      alternateDestination: "/restored/file.txt",
    });
    expect(invoke).toHaveBeenCalledWith(COMMAND_RESTORE_ITEM, {
      args: {
        operationItemId: "item-1",
        alternateDestination: "/restored/file.txt",
      },
    });
    expect(res.restoreId).toBe("res-1");
    expect(res.restoredToPath).toBe("/restored/file.txt");
  });

  it("dispatches fetch_lifetime_reclamation_totals", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      pendingInTrashBytes: 500n,
      permanentlyReclaimedBytes: 1000n,
    });

    const totals = await invokeFetchLifetimeReclamationTotals();
    expect(invoke).toHaveBeenCalledWith(
      COMMAND_FETCH_LIFETIME_RECLAMATION_TOTALS,
    );
    expect(totals.pendingInTrashBytes).toBe(500n);
    expect(totals.permanentlyReclaimedBytes).toBe(1000n);
  });
});

describe("boundary event subscriptions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("subscribes to scan progress, verified count, warnings, outcomes, and completion", async () => {
    const unlistenMock = vi.fn();
    vi.mocked(listen).mockResolvedValue(
      unlistenMock as unknown as ReturnType<typeof listen> extends Promise<
        infer U
      >
        ? U
        : never,
    );

    const progressHandler = vi.fn();
    const verifiedHandler = vi.fn();
    const warningHandler = vi.fn();
    const outcomeHandler = vi.fn();
    const terminalHandler = vi.fn();

    const u1 = await subscribeScanProgress(progressHandler);
    const u2 = await subscribeVerifiedCount(verifiedHandler);
    const u3 = await subscribeCoverageWarning(warningHandler);
    const u4 = await subscribeExecutionOutcome(outcomeHandler);
    const u5 = await subscribeTerminalCompletion(terminalHandler);

    expect(listen).toHaveBeenCalledWith(
      EVENT_SCAN_PROGRESS,
      expect.any(Function),
    );
    expect(listen).toHaveBeenCalledWith(
      EVENT_VERIFIED_COUNT,
      expect.any(Function),
    );
    expect(listen).toHaveBeenCalledWith(
      EVENT_COVERAGE_WARNING,
      expect.any(Function),
    );
    expect(listen).toHaveBeenCalledWith(
      EVENT_EXECUTION_OUTCOME,
      expect.any(Function),
    );
    expect(listen).toHaveBeenCalledWith(
      EVENT_TERMINAL_COMPLETION,
      expect.any(Function),
    );

    u1();
    u2();
    u3();
    u4();
    u5();
    expect(unlistenMock).toHaveBeenCalledTimes(5);
  });
});

describe("boundary error discriminated union", () => {
  it("allows pattern matching against all 4 error kinds", () => {
    const errors: CommandError[] = [
      { kind: "permissionDenied", path: "/opt", reason: "Forbidden" },
      { kind: "pathVanished", path: "/tmp/foo" },
      { kind: "changedAfterReview", planId: "p1", details: "Mtime changed" },
      {
        kind: "unsupported",
        feature: "scan",
        reason: "Pending implementation",
      },
    ];

    const kinds = errors.map((err) => {
      switch (err.kind) {
        case "permissionDenied":
          return `denied:${err.path}`;
        case "pathVanished":
          return `vanished:${err.path}`;
        case "changedAfterReview":
          return `changed:${err.planId}`;
        case "unsupported":
          return `unsupported:${err.feature}`;
      }
    });

    expect(kinds).toEqual([
      "denied:/opt",
      "vanished:/tmp/foo",
      "changed:p1",
      "unsupported:scan",
    ]);
  });
});
