import { describe, it, expect } from "vitest";
import { renderToString } from "react-dom/server";
import { ApplicationsTab, HomeTab, HistoryTab } from "./App";
import type {
  HistoryOperationDetail,
  HistoryOperationSummary,
  LifetimeReclamationTotals,
} from "./types/bindings";

describe("ApplicationsTab", () => {
  it("surfaces strong and guessed evidence with different authority", () => {
    const html = renderToString(
      <ApplicationsTab
        initialInventory={{
          applications: [
            {
              bundleId: "com.acme.alpha",
              name: "Alpha",
              version: "1.0",
              installPath: "/Applications/Alpha.app",
              bundleFootprintBytes: 2048n,
              relatedFilesFootprintBytes: 4096n,
              combinedFootprintBytes: 6144n,
              isRunning: false,
              canRemove: true,
              bundleSelectedByDefault: true,
              removalState: "ready",
              removalExplanation: "Application is not running.",
              relatedFiles: [
                {
                  path: "/Users/test/Library/Caches/com.acme.alpha",
                  name: "com.acme.alpha",
                  location: "caches",
                  measuredFootprintBytes: 2048n,
                  safetyClass: "rebuildable",
                  selectedByDefault: true,
                  offeredForRemoval: true,
                  evidence: {
                    reason: "exactBundleIdentifier",
                    strength: "strong",
                    explanation: "Matched by bundle identifier",
                  },
                },
                {
                  path: "/Users/test/Library/Application Support/Alpha Notes",
                  name: "Alpha Notes",
                  location: "applicationSupport",
                  measuredFootprintBytes: 2048n,
                  safetyClass: "review",
                  selectedByDefault: false,
                  offeredForRemoval: true,
                  evidence: {
                    reason: "similarName",
                    strength: "guess",
                    explanation: "Name looks similar; this is a guess",
                  },
                },
              ],
            },
          ],
          orphans: [],
        }}
      />,
    );

    expect(html).toContain("Matched by bundle identifier");
    expect(html).toContain("strong evidence");
    expect(html).toContain("Name looks similar");
    expect(html).toContain("guess, review required");
  });

  it("shows why a running application cannot be removed", () => {
    const html = renderToString(
      <ApplicationsTab
        initialInventory={{
          applications: [
            {
              bundleId: "com.acme.running",
              name: "Running App",
              version: "1.0",
              installPath: "/Applications/Running App.app",
              bundleFootprintBytes: 2048n,
              relatedFilesFootprintBytes: 0n,
              combinedFootprintBytes: 2048n,
              isRunning: true,
              canRemove: false,
              bundleSelectedByDefault: false,
              removalState: "quitRequired",
              removalExplanation:
                "Quit this application before removing its bundle or related files.",
              relatedFiles: [],
            },
          ],
          orphans: [],
        }}
      />,
    );

    expect(html).toContain("Quit this application before removing");
    expect(html).toContain('data-testid="remove-com.acme.running"');
    expect(html).toContain("disabled");
  });
});

describe("two-distinct-totals claim", () => {
  it("renders distinct figures for trash and freed space without summing them", () => {
    const mockReport = {
      pendingInTrashBytes: 524288000, // 500 MB
      permanentlyReclaimedBytes: 786432000, // 750 MB
    };
    const html = renderToString(<HomeTab report={mockReport} />);
    expect(html).toContain(
      "Ready to move to Trash:<!-- --> <!-- -->500<!-- --> MB",
    );
    expect(html).toContain(
      "Space available after Trash is emptied:<!-- --> <!-- -->750<!-- --> MB",
    );

    // Ensure one is not a sum of both
    expect(html).not.toContain("1250 MB");
  });
});

describe("HistoryTab operations history and restore", () => {
  it("renders two lifetime figures separately and never sums them", () => {
    const mockTotals: LifetimeReclamationTotals = {
      pendingInTrashBytes: 524288000n, // 500 MB
      permanentlyReclaimedBytes: 1048576000n, // 1000 MB
    };

    const html = renderToString(
      <HistoryTab initialTotals={mockTotals} initialOperations={[]} />,
    );

    expect(html).toContain('data-testid="lifetime-pending-trash"');
    expect(html).toContain('data-testid="lifetime-permanently-reclaimed"');
    expect(html).toContain("500 MB");
    expect(html).toContain("1000 MB");
    expect(html).not.toContain("1500 MB");
  });

  it("renders permanently deleted items without any restore button or affordance", () => {
    const mockOp: HistoryOperationSummary = {
      id: "op-perm-1",
      planId: "p-perm-1",
      actionMode: "permanentDelete",
      createdAtMs: 1000n,
      completedAtMs: 1005n,
      succeededItems: 1n,
      failedItems: 0n,
      skippedItems: 0n,
      blockedItems: 0n,
      vanishedItems: 0n,
      permissionDeniedItems: 0n,
      bytesPendingTrash: 0n,
      bytesPermanentlyReclaimed: 2048n,
      totalItems: 1n,
    };

    const mockDetail: HistoryOperationDetail = {
      operation: mockOp,
      items: [
        {
          actionType: "permanentDelete",
          operationItemId: "item-perm-1",
          itemId: "item-1",
          originalPath: "/path/to/destroyed.log",
          sizeBytes: 2048n,
          status: "succeeded",
          bytesPermanentlyReclaimed: 2048n,
          errorMessage: null,
          createdAtMs: 1000n,
        },
      ],
    };

    const html = renderToString(<HistoryTab initialDetail={mockDetail} />);

    expect(html).toContain('data-testid="item-item-perm-1"');
    expect(html).toContain("Permanently deleted");
    expect(html).toContain("/path/to/destroyed.log");
    // Strictly no restore button or action
    expect(html).not.toContain('data-testid="restore-btn-item-perm-1"');
    expect(html).not.toContain('data-testid="restore-alt-btn-item-perm-1"');
  });

  it("renders ineligible items with stated reason and keeps them visible in the list", () => {
    const mockOp: HistoryOperationSummary = {
      id: "op-trash-1",
      planId: "p-trash-1",
      actionMode: "trash",
      createdAtMs: 1000n,
      completedAtMs: 1005n,
      succeededItems: 1n,
      failedItems: 0n,
      skippedItems: 0n,
      blockedItems: 0n,
      vanishedItems: 0n,
      permissionDeniedItems: 0n,
      bytesPendingTrash: 1024n,
      bytesPermanentlyReclaimed: 0n,
      totalItems: 1n,
    };

    const mockDetail: HistoryOperationDetail = {
      operation: mockOp,
      items: [
        {
          actionType: "trash",
          operationItemId: "item-ineligible-1",
          itemId: "item-1",
          originalPath: "/path/to/emptied.cache",
          trashedPath: "/trash/emptied.cache",
          sizeBytes: 1024n,
          status: "succeeded",
          bytesPendingTrash: 1024n,
          errorMessage: null,
          createdAtMs: 1000n,
          restoreEligibility: {
            status: "ineligible",
            reason: "Item was emptied from trash",
          },
        },
      ],
    };

    const html = renderToString(<HistoryTab initialDetail={mockDetail} />);

    // Remains visible in the list
    expect(html).toContain('data-testid="item-item-ineligible-1"');
    expect(html).toContain("/path/to/emptied.cache");
    expect(html).toContain("Ineligible");
    expect(html).toContain("Item was emptied from trash");
    expect(html).not.toContain('data-testid="restore-btn-item-ineligible-1"');
  });

  it("warns about occupied destination and offers non-colliding alternate candidate", () => {
    const mockOp: HistoryOperationSummary = {
      id: "op-trash-2",
      planId: "p-trash-2",
      actionMode: "trash",
      createdAtMs: 1000n,
      completedAtMs: 1005n,
      succeededItems: 1n,
      failedItems: 0n,
      skippedItems: 0n,
      blockedItems: 0n,
      vanishedItems: 0n,
      permissionDeniedItems: 0n,
      bytesPendingTrash: 1024n,
      bytesPermanentlyReclaimed: 0n,
      totalItems: 1n,
    };

    const mockDetail: HistoryOperationDetail = {
      operation: mockOp,
      items: [
        {
          actionType: "trash",
          operationItemId: "item-colliding-1",
          itemId: "item-2",
          originalPath: "/path/to/document.txt",
          trashedPath: "/trash/document.txt",
          sizeBytes: 1024n,
          status: "succeeded",
          bytesPendingTrash: 1024n,
          errorMessage: null,
          createdAtMs: 1000n,
          restoreEligibility: {
            status: "eligible",
            trashed_path: "/trash/document.txt",
            original_destination: "/path/to/document.txt",
            destination_occupied: true,
            suggested_alternate_destination: "/path/to/document (Restored).txt",
          },
        },
      ],
    };

    const html = renderToString(<HistoryTab initialDetail={mockDetail} />);

    expect(html).toContain('data-testid="item-item-colliding-1"');
    expect(html).toContain("Destination occupied");
    expect(html).toContain('data-testid="restore-alt-btn-item-colliding-1"');
    expect(html).toContain("Restore as alternate");
    // Does NOT render plain restore button over occupied destination
    expect(html).not.toContain('data-testid="restore-btn-item-colliding-1"');
  });

  it("offers direct restore button when destination is unoccupied", () => {
    const mockOp: HistoryOperationSummary = {
      id: "op-trash-3",
      planId: "p-trash-3",
      actionMode: "trash",
      createdAtMs: 1000n,
      completedAtMs: 1005n,
      succeededItems: 1n,
      failedItems: 0n,
      skippedItems: 0n,
      blockedItems: 0n,
      vanishedItems: 0n,
      permissionDeniedItems: 0n,
      bytesPendingTrash: 1024n,
      bytesPermanentlyReclaimed: 0n,
      totalItems: 1n,
    };

    const mockDetail: HistoryOperationDetail = {
      operation: mockOp,
      items: [
        {
          actionType: "trash",
          operationItemId: "item-free-1",
          itemId: "item-3",
          originalPath: "/path/to/free.txt",
          trashedPath: "/trash/free.txt",
          sizeBytes: 1024n,
          status: "succeeded",
          bytesPendingTrash: 1024n,
          errorMessage: null,
          createdAtMs: 1000n,
          restoreEligibility: {
            status: "eligible",
            trashed_path: "/trash/free.txt",
            original_destination: "/path/to/free.txt",
            destination_occupied: false,
            suggested_alternate_destination: null,
          },
        },
      ],
    };

    const html = renderToString(<HistoryTab initialDetail={mockDetail} />);

    expect(html).toContain('data-testid="item-item-free-1"');
    expect(html).toContain('data-testid="restore-btn-item-free-1"');
    expect(html).toContain("Restore");
  });
});
