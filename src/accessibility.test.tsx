import { describe, expect, it } from "vitest";
import { renderToString } from "react-dom/server";
// @ts-expect-error type error without @types/node package
import { readFileSync } from "node:fs";
import {
  CleanupTab,
  ExploreTab,
  HomeTab,
  getVoiceOverFindingLabel,
  type FindingRowItem,
} from "./App";
import type { FolderAggregate } from "./types/bindings";

describe("Accessibility: Screen reader VoiceOver finding row labels", () => {
  const rebuildableItem: FindingRowItem = {
    id: "f-rebuildable",
    name: "Xcode DerivedData",
    path: "~/Library/Caches/DerivedData",
    sizeBytes: 2254857830n,
    safetyClass: "rebuildable",
    actionKind: "trash",
    recoverability: "Rebuildable cache",
  };

  const reviewItem: FindingRowItem = {
    id: "f-review",
    name: "Xcode Device Logs",
    path: "~/Library/Logs",
    sizeBytes: 1503238553n,
    safetyClass: "review",
    actionKind: "trash",
    recoverability: "Conditional recovery",
  };

  const protectedItem: FindingRowItem = {
    id: "f-protected",
    name: "Command Line Tools",
    path: "/Library/Developer/CommandLineTools",
    sizeBytes: 5153960755n,
    safetyClass: "protected",
    actionKind: "none",
    recoverability: "System protected",
  };

  it("announces name, size, class, selection, recoverability, and action for rebuildable items", () => {
    const selectedLabel = getVoiceOverFindingLabel(rebuildableItem, true);
    expect(selectedLabel).toContain("Xcode DerivedData");
    expect(selectedLabel).toContain("2.1 GB");
    expect(selectedLabel).toContain("Rebuildable cache");
    expect(selectedLabel).toContain("selected");
    expect(selectedLabel).toContain("press Space to deselect");
    expect(selectedLabel).toContain("press Enter to view evidence");

    const unselectedLabel = getVoiceOverFindingLabel(rebuildableItem, false);
    expect(unselectedLabel).toContain("unselected");
    expect(unselectedLabel).toContain("press Space to select");
  });

  it("announces name, size, class, selection, recoverability, and action for review items", () => {
    const label = getVoiceOverFindingLabel(reviewItem, false);
    expect(label).toContain("Xcode Device Logs");
    expect(label).toContain("1.4 GB");
    expect(label).toContain("Review required");
    expect(label).toContain("unselected");
    expect(label).toContain("conditional recovery");
    expect(label).toContain("press Space to select");
    expect(label).toContain("press Enter to view evidence");
  });

  it("announces uncheckable system protected status for protected items", () => {
    const label = getVoiceOverFindingLabel(protectedItem, false);
    expect(label).toContain("Command Line Tools");
    expect(label).toContain("4.8 GB");
    expect(label).toContain("Protected by system policy");
    expect(label).toContain("uncheckable");
    expect(label).toContain("cannot be modified or removed");
    expect(label).toContain("press Enter to view protection evidence");
  });
});

describe("Accessibility: Status never depends on colour alone", () => {
  it("renders both icon glyph and text label for every safety classification", () => {
    const html = renderToString(<CleanupTab />);
    // Rebuildable
    expect(html).toContain("↺");
    expect(html).toContain("Rebuildable");
    // Review
    expect(html).toContain("◇");
    expect(html).toContain("Review");
    // Protected
    expect(html).toContain("🔒");
    expect(html).toContain("Protected");
  });
});

describe("Accessibility: HomeTab controls and live announcements", () => {
  it("renders Scan Mac button with accessible label and throttled live scan region", () => {
    const html = renderToString(
      <HomeTab
        report={{ pendingInTrashBytes: 0, permanentlyReclaimedBytes: 0 }}
      />,
    );
    expect(html).toContain('data-testid="scan-mac-btn"');
    expect(html).toContain('aria-label="Scan Mac storage"');
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-live="polite"');
    expect(html).toContain('aria-atomic="true"');
  });
});

describe("Accessibility: CleanupTab finding rows and review tray", () => {
  it("renders finding rows with keyboard accessibility and evidence disclosures", () => {
    const html = renderToString(<CleanupTab />);
    expect(html).toContain('role="article"');
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain('aria-label="Show evidence for Xcode DerivedData"');
  });

  it("never auto-selects Review or Protected items", () => {
    const mockFindings: FindingRowItem[] = [
      {
        id: "r1",
        name: "Rebuildable Cache",
        path: "/path/cache",
        sizeBytes: 1000n,
        safetyClass: "rebuildable",
        actionKind: "trash",
        recoverability: "Rebuildable",
      },
      {
        id: "v1",
        name: "Review Log",
        path: "/path/log",
        sizeBytes: 2000n,
        safetyClass: "review",
        actionKind: "trash",
        recoverability: "Review",
      },
      {
        id: "p1",
        name: "Protected Root",
        path: "/path/root",
        sizeBytes: 3000n,
        safetyClass: "protected",
        actionKind: "none",
        recoverability: "Protected",
      },
    ];

    const html = renderToString(<CleanupTab initialFindings={mockFindings} />);
    // Review item checkbox is unchecked
    expect(html).toContain('data-testid="finding-row-v1"');
    expect(html).toContain('data-testid="finding-row-p1"');
    // Protected item checkbox is disabled
    expect(html).toContain('disabled="" aria-label="Select Protected Root"');
    // Review breakdown strictly specifies 0 Review items selected by default
    expect(html).toContain(
      "1<!-- --> items selected (<!-- -->1<!-- --> Rebuildable,<!-- --> <!-- -->0<!-- --> Review, 0 Protected)",
    );
  });

  it("renders docked review tray with segregated dual storage figures", () => {
    const html = renderToString(<CleanupTab />);
    expect(html).toContain('data-testid="review-tray"');
    expect(html).toContain('data-testid="tray-ready-trash"');
    expect(html).toContain('data-testid="tray-after-empty"');
    expect(html).toContain('data-testid="clear-selection-btn"');
    expect(html).toContain('data-testid="open-trash-sheet-btn"');
    expect(html).toContain('data-testid="open-perm-delete-sheet-btn"');
  });
});

describe("Accessibility: ExploreTab treemap and folder table parity", () => {
  const testAggregate: FolderAggregate = {
    path: "/test/dir",
    totalSizeBytes: 30000000000n,
    fileCount: 500n,
    children: [
      {
        name: "DerivedData",
        path: "/test/dir/DerivedData",
        sizeBytes: 20000000000n,
        fileCount: 400n,
        isDir: true,
      },
      {
        name: "CommandLineTools",
        path: "/test/dir/CommandLineTools",
        sizeBytes: 10000000000n,
        fileCount: 100n,
        isDir: true,
      },
    ],
  };

  it("provides view switcher between treemap and folder list representation", () => {
    const html = renderToString(
      <ExploreTab initialAggregate={testAggregate} initialViewMode="treemap" />,
    );
    expect(html).toContain('data-testid="explore-view-switcher"');
    expect(html).toContain('data-testid="switch-to-treemap"');
    expect(html).toContain('data-testid="switch-to-list"');
  });

  it("renders interactive treemap with accessible tile attributes", () => {
    const html = renderToString(
      <ExploreTab initialAggregate={testAggregate} initialViewMode="treemap" />,
    );
    expect(html).toContain('data-testid="explore-treemap"');
    expect(html).toContain('data-testid="treemap-tile-DerivedData"');
    expect(html).toContain('data-testid="treemap-tile-CommandLineTools"');
    expect(html).toContain('role="button"');
    expect(html).toContain('tabindex="0"');
    expect(html).toContain("DerivedData directory");
  });

  it("renders navigable folder table representing identical items for accessibility", () => {
    const html = renderToString(
      <ExploreTab initialAggregate={testAggregate} initialViewMode="list" />,
    );
    expect(html).toContain('data-testid="explore-folder-table-wrapper"');
    expect(html).toContain('data-testid="explore-folder-table"');
    expect(html).toContain('data-testid="folder-row-DerivedData"');
    expect(html).toContain('data-testid="folder-row-CommandLineTools"');
    expect(html).toContain('scope="col"');
    expect(html).toContain("DerivedData, folder");
    expect(html).toContain("CommandLineTools, folder");
  });

  it("renders contextual inspector with item metadata and action buttons", () => {
    const html = renderToString(
      <ExploreTab initialAggregate={testAggregate} initialViewMode="list" />,
    );
    expect(html).toContain('data-testid="explore-inspector"');
    expect(html).toContain('data-testid="inspector-add-plan-btn"');
    expect(html).toContain('data-testid="inspector-reveal-btn"');
  });
});

describe("Accessibility: Confirmation sheets name action and consequence in words", () => {
  it("renders Move to Trash sheet with explicit consequence and initial focus on safe action", () => {
    const html = renderToString(<CleanupTab initialActiveSheet="trash" />);

    expect(html).toContain('data-testid="trash-confirmation-sheet"');
    expect(html).toContain('role="dialog"');
    expect(html).toContain('aria-modal="true"');
    expect(html).toContain('id="trash-dialog-title"');
    expect(html).toContain("Move to Trash");

    // Consequence stated in plain words, never relying on color alone
    expect(html).toContain(
      "Files will be moved to macOS Trash. Disk space is not freed until Trash is emptied. Items remain recoverable in Finder until emptied.",
    );
    expect(html).toContain("Reclaimed immediately: 0 B");
    expect(html).toContain('data-testid="trash-cancel-btn"');
    expect(html).toContain('data-testid="trash-confirm-btn"');
  });

  it("renders Permanently Delete sheet with irreversible alert and disabled button until acknowledged", () => {
    const unacknowledgedHtml = renderToString(
      <CleanupTab
        initialActiveSheet="permanentDelete"
        initialAcknowledged={false}
      />,
    );

    expect(unacknowledgedHtml).toContain(
      'data-testid="perm-confirmation-sheet"',
    );
    expect(unacknowledgedHtml).toContain('role="dialog"');
    expect(unacknowledgedHtml).toContain('aria-modal="true"');
    expect(unacknowledgedHtml).toContain('data-testid="irreversible-warning"');
    expect(unacknowledgedHtml).toContain("Irreversible Permanent Destruction");

    // Consequence stated in words
    expect(unacknowledgedHtml).toContain(
      "Files will be deleted immediately and permanently. This action cannot be undone and these files cannot be recovered.",
    );
    expect(unacknowledgedHtml).toContain("Pending in Trash: 0 B");

    // Checkbox and disabled button state
    expect(unacknowledgedHtml).toContain('data-testid="perm-ack-checkbox"');
    expect(unacknowledgedHtml).toContain('disabled=""');
    expect(unacknowledgedHtml).toContain('data-testid="perm-confirm-btn"');

    // When acknowledged, delete button is enabled
    const acknowledgedHtml = renderToString(
      <CleanupTab
        initialActiveSheet="permanentDelete"
        initialAcknowledged={true}
      />,
    );
    expect(acknowledgedHtml).toContain('data-testid="perm-ack-checkbox"');
    // Button is not disabled
    expect(acknowledgedHtml).not.toMatch(
      /data-testid="perm-confirm-btn"[^>]*disabled=""/,
    );
  });
});

describe("Responsive conformance: 760x560 minimum window and content clearance", () => {
  it("verifies review tray has dedicated clearance space preventing finding occlusion", () => {
    const css = readFileSync("src/App.css", "utf-8") as string;

    // The content list must have at least 120px bottom clearance for the docked tray
    expect(css).toMatch(/\.cleanup-container[\s\S]*?padding-bottom:\s*120px/);
    expect(css).toMatch(
      /\.cleanup-container[\s\S]*?scroll-padding-bottom:\s*120px/,
    );

    // Sidebar collapses at <= 760px compact breakpoint
    expect(css).toMatch(
      /@media\s*\(max-width:\s*760px\)[\s\S]*?\.window-shell[\s\S]*?grid-template-columns:\s*72px\s*1fr/,
    );

    // Low-priority columns move to disclosure at compact breakpoint
    expect(css).toMatch(
      /@media\s*\(max-width:\s*760px\)[\s\S]*?\.col-last-activity[\s\S]*?display:\s*none/,
    );
    expect(css).toMatch(
      /@media\s*\(max-width:\s*760px\)[\s\S]*?\.col-recoverability[\s\S]*?display:\s*none/,
    );
  });
});
