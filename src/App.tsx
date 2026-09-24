import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  invokeFetchHistoryOperations,
  invokeFetchLifetimeReclamationTotals,
  invokeFetchOperationDetail,
  invokeRestoreItem,
} from "./boundary/client";
import type {
  FolderAggregate,
  FolderAggregateEntry,
  HistoryOperationDetail,
  HistoryOperationSummary,
  LifetimeReclamationTotals,
  SafetyClass,
} from "./types/bindings";
import "./App.css";

// Basic fallback
const fallbackFoundation = {
  status: "Offline",
  scanningImplemented: false,
  deletionImplemented: false,
};

export function formatBytes(bytes: bigint | number): string {
  const b = typeof bytes === "bigint" ? Number(bytes) : bytes;
  if (b === 0) return "0 B";
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) {
    const kb = b / 1024;
    return `${Number(kb.toFixed(1))} KB`;
  }
  if (b < 1024 * 1024 * 1024) {
    const mb = b / (1024 * 1024);
    return `${Number(mb.toFixed(1))} MB`;
  }
  const gb = b / (1024 * 1024 * 1024);
  return `${Number(gb.toFixed(2))} GB`;
}

export interface FindingRowItem {
  id: string;
  name: string;
  path: string;
  sizeBytes: bigint | number;
  safetyClass: SafetyClass;
  actionKind: string;
  recoverability: string;
  lastActivity?: string;
  owningTool?: string;
  ruleId?: string;
  regenerator?: string;
}

export function getVoiceOverFindingLabel(
  item: FindingRowItem,
  isSelected: boolean,
): string {
  const size = formatBytes(item.sizeBytes);
  if (item.safetyClass === "protected") {
    return `${item.name}, Protected by system policy, ${size}, uncheckable, essential system developer software, cannot be modified or removed, press Enter to view protection evidence`;
  }
  if (item.safetyClass === "review") {
    return `${item.name}, Review required, ${size}, ${isSelected ? "selected" : "unselected"}, conditional recovery, press Space to ${isSelected ? "deselect" : "select"}, press Enter to view evidence`;
  }
  return `${item.name}, Rebuildable cache, ${size}, ${isSelected ? "selected" : "unselected"}, rebuildable by tool, press Space to ${isSelected ? "deselect" : "select"}, press Enter to view evidence`;
}

export default function App() {
  const [activeTab, setActiveTab] = useState("Home");
  const [foundation, setFoundation] = useState(fallbackFoundation);
  const [report, setReport] = useState({
    pendingInTrashBytes: 0,
    permanentlyReclaimedBytes: 0,
  });

  useEffect(() => {
    invoke("foundation_status")
      .then((res: any) => setFoundation(res))
      .catch(() => setFoundation(fallbackFoundation));

    // Fetch a default session report if any exists
    invoke("fetch_storage_reclamation_report", { sessionId: "default" })
      .then((res: any) => setReport(res))
      .catch(() => {});
  }, []);

  const navItems = [
    { name: "Home", icon: "⌂" },
    { name: "Cleanup", icon: "↺" },
    { name: "Explore", icon: "⊞" },
    { name: "Applications", icon: "◈" },
    { name: "History", icon: "◷" },
  ];

  return (
    <main className="window-shell">
      <aside className="sidebar" aria-label="Primary navigation">
        <div className="brand-mark" aria-hidden="true">
          <span />
        </div>
        <strong>DiskClearance</strong>
        <nav role="navigation" aria-label="Main menu">
          {navItems.map((item) => (
            <button
              key={item.name}
              className={`nav-item ${activeTab === item.name ? "active" : ""}`}
              type="button"
              disabled={item.name === "Applications"}
              aria-current={activeTab === item.name ? "page" : undefined}
              onClick={() => setActiveTab(item.name)}
            >
              <span className="nav-icon" aria-hidden="true">
                {item.icon}
              </span>
              <span className="nav-label">{item.name}</span>
              {item.name === "Applications" && <small>Planned</small>}
            </button>
          ))}
        </nav>
      </aside>
      <section className="content">
        <div className="status-pill" style={{ display: "none" }}>
          {foundation.status}
        </div>
        {activeTab === "Home" && <HomeTab report={report} />}
        {activeTab === "Cleanup" && <CleanupTab />}
        {activeTab === "Explore" && <ExploreTab />}
        {activeTab === "History" && <HistoryTab />}
      </section>
    </main>
  );
}

export function HomeTab({
  report,
  onStartScan,
}: {
  report: { pendingInTrashBytes: number; permanentlyReclaimedBytes: number };
  onStartScan?: () => void;
}) {
  return (
    <div data-testid="home-tab" className="home-tab">
      <h1>See what goes. Keep what matters.</h1>
      <div>
        <div data-testid="reclaim-trash">
          Ready to move to Trash:{" "}
          {Math.round(report.pendingInTrashBytes / 1024 / 1024)} MB
        </div>
        <div data-testid="reclaim-freed">
          Space available after Trash is emptied:{" "}
          {Math.round(report.permanentlyReclaimedBytes / 1024 / 1024)} MB
        </div>
      </div>
      <button
        type="button"
        className="action-btn primary scan-mac-btn"
        data-testid="scan-mac-btn"
        aria-label="Scan Mac storage"
        onClick={onStartScan}
      >
        Scan Mac
      </button>
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="scan-live-announcer sr-only"
        data-testid="live-scan-announcements"
      >
        Ready to scan. Storage inspection is read-only until explicit
        confirmation.
      </div>
    </div>
  );
}

const defaultCleanupFindings: FindingRowItem[] = [
  {
    id: "finding-deriveddata",
    name: "Xcode DerivedData",
    path: "~/Library/Caches/com.synthetic.developer/DerivedData",
    sizeBytes: 2254857830n, // ~2.10 GB
    safetyClass: "rebuildable",
    actionKind: "trash",
    recoverability: "Rebuildable cache",
    lastActivity: "34 days ago",
    owningTool: "Xcode 15.2",
    ruleId: "developer.xcode.deriveddata",
    regenerator: "Rebuilt automatically by Xcode on next build.",
  },
  {
    id: "finding-simulators",
    name: "Xcode Simulator Caches",
    path: "~/Library/Caches/com.synthetic.developer/Simulator",
    sizeBytes: 2147483648n, // ~2.00 GB
    safetyClass: "rebuildable",
    actionKind: "trash",
    recoverability: "Rebuildable cache",
    lastActivity: "12 days ago",
    owningTool: "Xcode 15.2",
    ruleId: "developer.xcode.simulator",
    regenerator: "Rebuilt on next simulator launch.",
  },
  {
    id: "finding-devicelogs",
    name: "Xcode Device Logs",
    path: "~/Library/Application Support/SyntheticStudio/Logs",
    sizeBytes: 1503238553n, // ~1.40 GB
    safetyClass: "review",
    actionKind: "trash",
    recoverability: "Conditional recovery",
    lastActivity: "3 months ago",
    owningTool: "SyntheticStudio",
    ruleId: "developer.logs",
    regenerator:
      "Not rebuilt automatically; review diagnostic logs before clearing.",
  },
  {
    id: "finding-tools",
    name: "Command Line Tools",
    path: "/Library/Developer/CommandLineTools",
    sizeBytes: 5153960755n, // ~4.80 GB
    safetyClass: "protected",
    actionKind: "none",
    recoverability: "System protected",
    lastActivity: "System install",
    owningTool: "macOS Developer",
    ruleId: "system.developer.tools",
    regenerator: "Essential developer software; cannot be modified or removed.",
  },
];

export function CleanupTab({
  initialFindings,
  initialTrashBytes,
  initialSelected,
  initialActiveSheet = null,
  initialAcknowledged = false,
}: {
  initialFindings?: FindingRowItem[];
  initialTrashBytes?: bigint | number;
  initialSelected?: Record<string, boolean>;
  initialActiveSheet?: null | "trash" | "permanentDelete";
  initialAcknowledged?: boolean;
} = {}) {
  const [findings, setFindings] = useState<FindingRowItem[]>(
    initialFindings || defaultCleanupFindings,
  );
  const [selected, setSelected] = useState<Record<string, boolean>>(() => {
    if (initialSelected) return initialSelected;
    const initial: Record<string, boolean> = {};
    const items = initialFindings || defaultCleanupFindings;
    items.forEach((item) => {
      // Rebuildable items selected by default; Review and Protected are NEVER selected by default
      if (item.safetyClass === "rebuildable") {
        initial[item.id] = true;
      }
    });
    return initial;
  });
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [activeSheet, setActiveSheet] = useState<
    null | "trash" | "permanentDelete"
  >(initialActiveSheet);
  const [acknowledged, setAcknowledged] = useState(initialAcknowledged);

  const moveTrashTriggerRef = useRef<HTMLButtonElement | null>(null);
  const permDeleteTriggerRef = useRef<HTMLButtonElement | null>(null);
  const modalPrimaryTrashBtnRef = useRef<HTMLButtonElement | null>(null);
  const modalCancelPermBtnRef = useRef<HTMLButtonElement | null>(null);
  const modalDialogRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!initialFindings) {
      invoke("fetch_findings_page", { sessionId: "default", limit: 50 })
        .then((res: any) => {
          if (res?.items && res.items.length > 0) {
            const mapped: FindingRowItem[] = res.items.map((item: any) => {
              const sc: SafetyClass =
                item.actionKind === "permanentDelete"
                  ? "review"
                  : item.category === "protected"
                    ? "protected"
                    : "rebuildable";
              return {
                id: item.id,
                name: item.path.split("/").pop() || item.id,
                path: item.path,
                sizeBytes: BigInt(item.sizeBytes),
                safetyClass: sc,
                actionKind: item.actionKind,
                recoverability:
                  sc === "rebuildable"
                    ? "Rebuildable cache"
                    : sc === "protected"
                      ? "System protected"
                      : "Conditional recovery",
              };
            });
            setFindings(mapped);
          }
        })
        .catch(() => {});
    }
  }, [initialFindings]);

  // Focus management when sheet opens
  useEffect(() => {
    if (activeSheet === "trash") {
      modalPrimaryTrashBtnRef.current?.focus();
    } else if (activeSheet === "permanentDelete") {
      modalCancelPermBtnRef.current?.focus();
    }
  }, [activeSheet]);

  const toggleSelect = (id: string, safetyClass: SafetyClass) => {
    if (safetyClass === "protected") return;
    setSelected((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const toggleExpand = (id: string) => {
    setExpanded((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const handleRowKeyDown = (e: React.KeyboardEvent, item: FindingRowItem) => {
    if (e.key === " ") {
      e.preventDefault();
      toggleSelect(item.id, item.safetyClass);
    } else if (e.key === "Enter") {
      e.preventDefault();
      toggleExpand(item.id);
    }
  };

  const openSheet = (mode: "trash" | "permanentDelete") => {
    setActiveSheet(mode);
    setAcknowledged(false);
  };

  const closeSheet = () => {
    const closedMode = activeSheet;
    setActiveSheet(null);
    if (closedMode === "trash") {
      moveTrashTriggerRef.current?.focus();
    } else if (closedMode === "permanentDelete") {
      permDeleteTriggerRef.current?.focus();
    }
  };

  // Traps focus inside the modal dialog
  const handleModalKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      closeSheet();
      return;
    }

    if (e.key === "Tab" && modalDialogRef.current) {
      const focusable = modalDialogRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), [tabindex="0"]',
      );
      if (focusable.length > 0) {
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
  };

  const handleConfirmTrash = () => {
    closeSheet();
  };

  const handleConfirmPermanentDelete = () => {
    if (!acknowledged) return;
    closeSheet();
  };

  // Running totals calculation
  let selectedBytes = 0n;
  let selectedCount = 0;
  let rebuildableCount = 0;
  let reviewCount = 0;

  findings.forEach((f) => {
    if (selected[f.id]) {
      const b =
        typeof f.sizeBytes === "bigint" ? f.sizeBytes : BigInt(f.sizeBytes);
      selectedBytes += b;
      selectedCount += 1;
      if (f.safetyClass === "rebuildable") rebuildableCount += 1;
      if (f.safetyClass === "review") reviewCount += 1;
    }
  });

  const trashBytes =
    initialTrashBytes !== undefined
      ? typeof initialTrashBytes === "bigint"
        ? initialTrashBytes
        : BigInt(initialTrashBytes)
      : 8589934592n; // 8.00 GB default existing Trash

  const totalAfterEmpty = trashBytes + selectedBytes;

  return (
    <div data-testid="cleanup-tab" className="cleanup-container">
      <header>
        <h2>Actionable Findings</h2>
        <p className="lede">
          Review safe targets for removal. System protected items cannot be
          selected.
        </p>
      </header>

      <ul className="finding-list" aria-label="Actionable findings list">
        {findings.map((f) => {
          const isSelected = !!selected[f.id];
          const isExpanded = !!expanded[f.id];
          const isProtected = f.safetyClass === "protected";

          return (
            <li key={f.id} className="finding-row-container">
              <div
                className="finding-row"
                tabIndex={0}
                role="article"
                aria-label={getVoiceOverFindingLabel(f, isSelected)}
                data-testid={`finding-row-${f.id}`}
                onKeyDown={(e) => handleRowKeyDown(e, f)}
              >
                <div className="col-checkbox">
                  <input
                    type="checkbox"
                    className="row-checkbox"
                    checked={isSelected}
                    disabled={isProtected}
                    aria-label={`Select ${f.name}`}
                    tabIndex={0}
                    onChange={() => toggleSelect(f.id, f.safetyClass)}
                    onClick={(e) => e.stopPropagation()}
                  />
                </div>
                <div className="col-name">{f.name}</div>
                <div className="col-badge">
                  <span className={`safety-badge ${f.safetyClass}`}>
                    <span aria-hidden="true">
                      {f.safetyClass === "rebuildable"
                        ? "↺"
                        : f.safetyClass === "review"
                          ? "◇"
                          : "🔒"}
                    </span>{" "}
                    {f.safetyClass === "rebuildable"
                      ? "Rebuildable"
                      : f.safetyClass === "review"
                        ? "Review"
                        : "Protected"}
                  </span>
                </div>
                {f.lastActivity && (
                  <div className="col-last-activity">{f.lastActivity}</div>
                )}
                {f.recoverability && (
                  <div className="col-recoverability">{f.recoverability}</div>
                )}
                <div className="col-size storage-figure">
                  {formatBytes(f.sizeBytes)}
                </div>
                <button
                  type="button"
                  className="disclosure-btn"
                  aria-expanded={isExpanded}
                  aria-controls={`evidence-${f.id}`}
                  aria-label={`${isExpanded ? "Hide" : "Show"} evidence for ${f.name}`}
                  onClick={() => toggleExpand(f.id)}
                >
                  <span aria-hidden="true">{isExpanded ? "▼" : "▶"}</span>
                </button>
              </div>

              {isExpanded && (
                <div
                  id={`evidence-${f.id}`}
                  className="evidence-drawer"
                  role="region"
                  aria-label={`Evidence details for ${f.name}`}
                  data-testid={`evidence-${f.id}`}
                >
                  <div className="evidence-field">
                    <span className="evidence-label">Rule identifier:</span>
                    <code>{f.ruleId || "developer.xcode.cache"}</code>
                  </div>
                  {f.owningTool && (
                    <div className="evidence-field">
                      <span className="evidence-label">Owning tool:</span>
                      <span>{f.owningTool}</span>
                    </div>
                  )}
                  <div className="evidence-field">
                    <span className="evidence-label">Canonical path:</span>
                    <span className="evidence-path">{f.path}</span>
                  </div>
                  {f.regenerator && (
                    <div className="evidence-field">
                      <span className="evidence-label">Regeneration:</span>
                      <span>{f.regenerator}</span>
                    </div>
                  )}
                  <div className="evidence-field">
                    <span className="evidence-label">Recoverability:</span>
                    <span>
                      {f.recoverability}.{" "}
                      {f.safetyClass === "rebuildable"
                        ? "Moving to Trash preserves items until emptied in Finder."
                        : f.safetyClass === "protected"
                          ? "Protected by system policy and cannot be altered."
                          : "Requires manual inspection before clearing."}
                    </span>
                  </div>
                </div>
              )}
            </li>
          );
        })}
      </ul>

      {/* Docked Review Tray — Fixed at bottom, never occluding content */}
      <footer
        className="review-tray"
        role="region"
        aria-label="Selection review tray"
        data-testid="review-tray"
      >
        <div className="review-tray-info">
          <div className="tray-badge" data-testid="class-breakdown">
            {selectedCount} items selected ({rebuildableCount} Rebuildable,{" "}
            {reviewCount} Review, 0 Protected)
          </div>
          <div className="review-totals">
            <div>
              Ready to move to Trash:{" "}
              <span
                className="tray-figure storage-figure"
                data-testid="tray-ready-trash"
              >
                {formatBytes(selectedBytes)}
              </span>
            </div>
            <div>
              Space after Trash empty:{" "}
              <span
                className="tray-figure storage-figure"
                data-testid="tray-after-empty"
              >
                {formatBytes(totalAfterEmpty)}
              </span>
            </div>
          </div>
        </div>

        <div className="tray-actions">
          <button
            type="button"
            className="btn-secondary"
            onClick={() => setSelected({})}
            data-testid="clear-selection-btn"
          >
            Clear selection
          </button>
          <button
            type="button"
            ref={moveTrashTriggerRef}
            className="btn-primary"
            aria-haspopup="dialog"
            onClick={() => openSheet("trash")}
            data-testid="open-trash-sheet-btn"
          >
            Move to Trash…
          </button>
          <button
            type="button"
            ref={permDeleteTriggerRef}
            className="btn-secondary"
            aria-haspopup="dialog"
            aria-label="Permanently delete…"
            onClick={() => openSheet("permanentDelete")}
            data-testid="open-perm-delete-sheet-btn"
          >
            ⋯ Permanently delete…
          </button>
        </div>
      </footer>

      {/* Confirmation Sheet: Move to Trash */}
      {activeSheet === "trash" && (
        <div className="sheet-backdrop" onClick={closeSheet}>
          <div
            ref={modalDialogRef}
            className="confirmation-sheet"
            role="dialog"
            aria-modal="true"
            aria-labelledby="trash-dialog-title"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={handleModalKeyDown}
            data-testid="trash-confirmation-sheet"
          >
            <div className="sheet-header">
              <h2 id="trash-dialog-title">Move to Trash</h2>
              <button
                type="button"
                className="close-btn"
                aria-label="Close dialog"
                onClick={closeSheet}
              >
                ✕
              </button>
            </div>
            <div className="sheet-body">
              <p className="consequence-text">
                Files will be moved to macOS Trash. Disk space is not freed
                until Trash is emptied. Items remain recoverable in Finder until
                emptied.
              </p>
              <div className="storage-impact-box">
                <div>Ready to move to Trash: {formatBytes(selectedBytes)}</div>
                <div>Reclaimed immediately: 0 B</div>
                <div>
                  Space after Trash empty: {formatBytes(totalAfterEmpty)}
                </div>
              </div>
            </div>
            <div className="sheet-footer">
              <button
                type="button"
                className="btn-secondary"
                onClick={closeSheet}
                data-testid="trash-cancel-btn"
              >
                Cancel
              </button>
              <button
                type="button"
                ref={modalPrimaryTrashBtnRef}
                className="btn-primary"
                onClick={handleConfirmTrash}
                data-testid="trash-confirm-btn"
              >
                Move to Trash
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Confirmation Sheet: Permanently Delete */}
      {activeSheet === "permanentDelete" && (
        <div className="sheet-backdrop" onClick={closeSheet}>
          <div
            ref={modalDialogRef}
            className="confirmation-sheet irreversible"
            role="dialog"
            aria-modal="true"
            aria-labelledby="perm-dialog-title"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={handleModalKeyDown}
            data-testid="perm-confirmation-sheet"
          >
            <div className="sheet-header">
              <h2 id="perm-dialog-title">
                Permanently delete {selectedCount} items
              </h2>
              <button
                type="button"
                className="close-btn"
                aria-label="Close dialog"
                onClick={closeSheet}
              >
                ✕
              </button>
            </div>
            <div className="sheet-body">
              <div
                className="irreversible-warning-banner"
                role="alert"
                data-testid="irreversible-warning"
              >
                <span aria-hidden="true">⚠</span> Irreversible Permanent
                Destruction
              </div>
              <p className="consequence-text">
                Files will be deleted immediately and permanently. This action
                cannot be undone and these files cannot be recovered.
              </p>
              <div className="storage-impact-box">
                <div>
                  Permanently reclaimed immediately:{" "}
                  {formatBytes(selectedBytes)}
                </div>
                <div>Pending in Trash: 0 B</div>
              </div>
              <label className="ack-label">
                <input
                  type="checkbox"
                  checked={acknowledged}
                  onChange={(e) => setAcknowledged(e.target.checked)}
                  data-testid="perm-ack-checkbox"
                />
                <span>
                  I understand that these files will be permanently destroyed
                  and cannot be recovered
                </span>
              </label>
            </div>
            <div className="sheet-footer">
              <button
                type="button"
                ref={modalCancelPermBtnRef}
                className="btn-secondary"
                onClick={closeSheet}
                data-testid="perm-cancel-btn"
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={!acknowledged}
                className="btn-danger"
                onClick={handleConfirmPermanentDelete}
                data-testid="perm-confirm-btn"
              >
                Delete Now Permanently
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

const defaultExploreAggregate: FolderAggregate = {
  path: "~/Developer",
  totalSizeBytes: 47244640256n, // ~44 GB
  fileCount: 6382n,
  children: [
    {
      name: "Projects",
      path: "~/Developer/Projects",
      sizeBytes: 19756849561n, // ~18.4 GB
      fileCount: 14n,
      isDir: true,
    },
    {
      name: "DerivedData",
      path: "~/Developer/DerivedData",
      sizeBytes: 13529146982n, // ~12.6 GB
      fileCount: 4812n,
      isDir: true,
    },
    {
      name: "Simulators",
      path: "~/Developer/Simulators",
      sizeBytes: 6657199308n, // ~6.2 GB
      fileCount: 1240n,
      isDir: true,
    },
    {
      name: "CommandLineTools",
      path: "~/Developer/CommandLineTools",
      sizeBytes: 5153960755n, // ~4.8 GB
      fileCount: 318n,
      isDir: true,
    },
    {
      name: "Archives",
      path: "~/Developer/Archives",
      sizeBytes: 2147483648n, // ~2.1 GB
      fileCount: 12n,
      isDir: true,
    },
  ],
};

function getFolderSafetyClass(name: string): SafetyClass {
  if (name === "DerivedData" || name === "Simulators") return "rebuildable";
  if (name === "CommandLineTools" || name === "Toolchains") return "protected";
  if (name === "Archives") return "review";
  return "rebuildable";
}

export function ExploreTab({
  initialAggregate,
  initialPath,
  initialViewMode,
}: {
  initialAggregate?: FolderAggregate;
  initialPath?: string;
  initialViewMode?: "treemap" | "list";
} = {}) {
  const [agg, setAgg] = useState<FolderAggregate>(
    initialAggregate || defaultExploreAggregate,
  );
  const [currentPath, setCurrentPath] = useState(initialPath || agg.path);
  const [viewMode, setViewMode] = useState<"treemap" | "list">(
    initialViewMode || "treemap",
  );
  const [selectedNode, setSelectedNode] = useState<FolderAggregateEntry | null>(
    agg.children[0] || null,
  );

  useEffect(() => {
    if (!initialAggregate) {
      invoke("fetch_folder_aggregate", {
        sessionId: "default",
        path: currentPath,
      })
        .then((res: any) => {
          if (res?.children) {
            setAgg(res);
            if (res.children.length > 0) {
              setSelectedNode(res.children[0]);
            }
          }
        })
        .catch(() => {});
    }
  }, [currentPath, initialAggregate]);

  const handleDrillDown = (entry: FolderAggregateEntry) => {
    if (entry.isDir) {
      setCurrentPath(entry.path);
    }
  };

  const handleNavigateUp = () => {
    const parts = currentPath.split("/").filter(Boolean);
    if (parts.length > 1) {
      parts.pop();
      setCurrentPath("/" + parts.join("/"));
    }
  };

  const breadcrumbs = currentPath.split("/").filter(Boolean);

  const activeEntry = selectedNode || agg.children[0] || null;
  const activeSafetyClass = activeEntry
    ? getFolderSafetyClass(activeEntry.name)
    : "rebuildable";

  return (
    <div data-testid="explore-tab" className="explore-container">
      <header className="explore-toolbar">
        {/* Breadcrumb Navigation */}
        <nav
          aria-label="Explore path breadcrumbs"
          className="breadcrumbs-nav"
          data-testid="explore-breadcrumbs"
        >
          <button
            type="button"
            className="breadcrumb-btn"
            onClick={() => setCurrentPath("/")}
          >
            Macintosh HD
          </button>
          {breadcrumbs.map((segment, index) => {
            const subPath = "/" + breadcrumbs.slice(0, index + 1).join("/");
            const isLast = index === breadcrumbs.length - 1;
            return (
              <span
                key={subPath}
                style={{ display: "inline-flex", alignItems: "center" }}
              >
                <span className="breadcrumb-separator" aria-hidden="true">
                  &gt;
                </span>
                <button
                  type="button"
                  className="breadcrumb-btn"
                  aria-current={isLast ? "location" : undefined}
                  onClick={() => setCurrentPath(subPath)}
                >
                  {segment}
                </button>
              </span>
            );
          })}
        </nav>

        {/* View Switcher: Treemap vs List */}
        <div
          role="group"
          aria-label="View representation"
          className="view-switcher"
          data-testid="explore-view-switcher"
        >
          <button
            type="button"
            className={`switcher-btn ${viewMode === "treemap" ? "active" : ""}`}
            aria-pressed={viewMode === "treemap"}
            onClick={() => setViewMode("treemap")}
            data-testid="switch-to-treemap"
          >
            Treemap
          </button>
          <button
            type="button"
            className={`switcher-btn ${viewMode === "list" ? "active" : ""}`}
            aria-pressed={viewMode === "list"}
            onClick={() => setViewMode("list")}
            data-testid="switch-to-list"
          >
            Folder List
          </button>
        </div>
      </header>

      <div className="explore-content-layout">
        {/* Treemap representation */}
        {viewMode === "treemap" ? (
          <div
            className="treemap-container"
            role="region"
            aria-label="Disk usage treemap"
            data-testid="explore-treemap"
          >
            {agg.children.map((child) => {
              const sc = getFolderSafetyClass(child.name);
              return (
                <div
                  key={child.name}
                  className={`treemap-tile ${sc}`}
                  tabIndex={0}
                  role="button"
                  data-testid={`treemap-tile-${child.name}`}
                  aria-label={`${child.name} ${child.isDir ? "directory" : "file"}, ${formatBytes(child.sizeBytes)}, ${child.fileCount.toString()} files, ${sc}, press Enter to drill down into this folder`}
                  onClick={() => {
                    setSelectedNode(child);
                    handleDrillDown(child);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === "ArrowDown") {
                      e.preventDefault();
                      handleDrillDown(child);
                    } else if (e.key === "Backspace" || e.key === "Escape") {
                      e.preventDefault();
                      handleNavigateUp();
                    } else if (e.key === "ArrowRight") {
                      const next = e.currentTarget
                        .nextElementSibling as HTMLElement;
                      next?.focus();
                    } else if (e.key === "ArrowLeft") {
                      const prev = e.currentTarget
                        .previousElementSibling as HTMLElement;
                      prev?.focus();
                    }
                  }}
                  onFocus={() => setSelectedNode(child)}
                >
                  <div className="tile-name">{child.name}</div>
                  <div className="tile-size storage-figure">
                    {formatBytes(child.sizeBytes)}
                  </div>
                  <span className={`safety-badge ${sc}`}>
                    <span aria-hidden="true">
                      {sc === "rebuildable"
                        ? "↺"
                        : sc === "review"
                          ? "◇"
                          : "🔒"}
                    </span>{" "}
                    {sc === "rebuildable"
                      ? "Rebuildable"
                      : sc === "review"
                        ? "Review"
                        : "Protected"}
                  </span>
                </div>
              );
            })}
          </div>
        ) : (
          /* Folder table representation (Accessible treemap equivalent) */
          <div
            className="folder-table-wrapper"
            role="region"
            aria-label="Folder contents table"
            data-testid="explore-folder-table-wrapper"
          >
            <table className="folder-table" data-testid="explore-folder-table">
              <thead>
                <tr>
                  <th scope="col">Name</th>
                  <th scope="col">Safety Class</th>
                  <th scope="col">Item Count</th>
                  <th scope="col">Size</th>
                </tr>
              </thead>
              <tbody>
                {agg.children.map((child) => {
                  const sc = getFolderSafetyClass(child.name);
                  return (
                    <tr
                      key={child.name}
                      tabIndex={0}
                      role="row"
                      data-testid={`folder-row-${child.name}`}
                      aria-label={`${child.name}, ${child.isDir ? "folder" : "file"}, ${formatBytes(child.sizeBytes)}, ${child.fileCount.toString()} files, ${sc}, press Enter to open folder`}
                      onClick={() => {
                        setSelectedNode(child);
                        handleDrillDown(child);
                      }}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          e.preventDefault();
                          handleDrillDown(child);
                        } else if (e.key === "Backspace") {
                          e.preventDefault();
                          handleNavigateUp();
                        } else if (e.key === "ArrowDown") {
                          const next = e.currentTarget
                            .nextElementSibling as HTMLElement;
                          next?.focus();
                        } else if (e.key === "ArrowUp") {
                          const prev = e.currentTarget
                            .previousElementSibling as HTMLElement;
                          prev?.focus();
                        }
                      }}
                      onFocus={() => setSelectedNode(child)}
                    >
                      <td>
                        <span aria-hidden="true">
                          {child.isDir ? "📁 " : "📄 "}
                        </span>
                        <strong>{child.name}</strong>
                      </td>
                      <td>
                        <span className={`safety-badge ${sc}`}>
                          <span aria-hidden="true">
                            {sc === "rebuildable"
                              ? "↺"
                              : sc === "review"
                                ? "◇"
                                : "🔒"}
                          </span>{" "}
                          {sc === "rebuildable"
                            ? "Rebuildable"
                            : sc === "review"
                              ? "Review"
                              : "Protected"}
                        </span>
                      </td>
                      <td className="tabular-nums">
                        {child.fileCount.toString()} files
                      </td>
                      <td className="tabular-nums storage-figure size-cell">
                        {formatBytes(child.sizeBytes)}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}

        {/* Contextual Inspector Panel */}
        {activeEntry && (
          <aside
            className="inspector-panel desktop-only"
            aria-label="Item details inspector"
            data-testid="explore-inspector"
          >
            <h3>{activeEntry.name}</h3>
            <div className="inspector-meta">
              <div>
                <strong>Path:</strong> <code>{activeEntry.path}</code>
              </div>
              <div>
                <strong>Size:</strong> {formatBytes(activeEntry.sizeBytes)}
              </div>
              <div>
                <strong>Items:</strong> {activeEntry.fileCount.toString()} files
              </div>
              <div>
                <span className={`safety-badge ${activeSafetyClass}`}>
                  <span aria-hidden="true">
                    {activeSafetyClass === "rebuildable"
                      ? "↺"
                      : activeSafetyClass === "review"
                        ? "◇"
                        : "🔒"}
                  </span>{" "}
                  {activeSafetyClass === "rebuildable"
                    ? "Rebuildable"
                    : activeSafetyClass === "review"
                      ? "Review"
                      : "Protected"}
                </span>
              </div>
            </div>

            <div className="inspector-actions">
              <button
                type="button"
                className="btn-primary"
                disabled={activeSafetyClass === "protected"}
                data-testid="inspector-add-plan-btn"
              >
                Add to Review Plan
              </button>
              <button
                type="button"
                className="btn-secondary"
                data-testid="inspector-reveal-btn"
              >
                Reveal in Finder
              </button>
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}

export function HistoryTab({
  initialTotals,
  initialOperations,
  initialDetail,
}: {
  initialTotals?: LifetimeReclamationTotals;
  initialOperations?: HistoryOperationSummary[];
  initialDetail?: HistoryOperationDetail;
} = {}) {
  const [totals, setTotals] = useState<LifetimeReclamationTotals>(
    initialTotals || {
      pendingInTrashBytes: 0n,
      permanentlyReclaimedBytes: 0n,
    },
  );
  const [operations, setOperations] = useState<HistoryOperationSummary[]>(
    initialOperations || [],
  );
  const [selectedOperationId, setSelectedOperationId] = useState<string | null>(
    initialDetail ? initialDetail.operation.id : null,
  );
  const [operationDetail, setOperationDetail] =
    useState<HistoryOperationDetail | null>(initialDetail || null);
  const [outcomeFilter, setOutcomeFilter] = useState<string>("all");
  const [restoreMessage, setRestoreMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const loadTotals = () => {
    invokeFetchLifetimeReclamationTotals()
      .then(setTotals)
      .catch(() => {});
  };

  const loadOperations = (filter: string) => {
    const filterArg = filter === "all" ? null : filter;
    invokeFetchHistoryOperations({
      outcomeFilter: filterArg,
      dateFromMs: null,
      dateToMs: null,
    })
      .then(setOperations)
      .catch((err) => {
        setErrorMessage(
          typeof err === "string" ? err : "Failed to load operations",
        );
      });
  };

  const loadDetail = (opId: string) => {
    invokeFetchOperationDetail({ operationId: opId, outcomeFilter: null })
      .then((detail) => {
        setOperationDetail(detail);
        setSelectedOperationId(opId);
      })
      .catch((err) => {
        setErrorMessage(
          typeof err === "string" ? err : "Failed to load operation detail",
        );
      });
  };

  useEffect(() => {
    if (!initialTotals) {
      loadTotals();
    }
  }, [initialTotals]);

  useEffect(() => {
    if (!initialOperations) {
      loadOperations(outcomeFilter);
    }
  }, [outcomeFilter, initialOperations]);

  const handleSelectOperation = (opId: string) => {
    setRestoreMessage(null);
    setErrorMessage(null);
    loadDetail(opId);
  };

  const handleBackToList = () => {
    setSelectedOperationId(null);
    setOperationDetail(null);
    setRestoreMessage(null);
    setErrorMessage(null);
    loadOperations(outcomeFilter);
  };

  const handleRestore = (
    operationItemId: string,
    alternateDestination: string | null,
  ) => {
    setRestoreMessage(null);
    setErrorMessage(null);
    invokeRestoreItem({ operationItemId, alternateDestination })
      .then((summary) => {
        setRestoreMessage(`Restored successfully to ${summary.restoredToPath}`);
        if (selectedOperationId) {
          loadDetail(selectedOperationId);
        }
        loadTotals();
      })
      .catch((err) => {
        setErrorMessage(
          typeof err === "string" ? err : (err?.reason ?? "Restore failed"),
        );
      });
  };

  return (
    <div data-testid="history-tab" className="history-tab">
      <header className="history-header">
        <h2>Operations History</h2>
        <p className="history-description">
          Review past clearance operations and safely restore eligible items
          from the Trash.
        </p>
      </header>

      {/* Two lifetime figures strictly separated */}
      <div
        className="history-totals-grid"
        role="region"
        aria-label="Lifetime reclamation totals"
      >
        <div className="stat-card" data-testid="lifetime-pending-trash">
          <span className="card-label">Currently Pending in Trash</span>
          <div className="storage-figure">
            {formatBytes(totals.pendingInTrashBytes)}
          </div>
          <p className="stat-hint">Reversible until Trash is emptied.</p>
        </div>

        <div className="stat-card" data-testid="lifetime-permanently-reclaimed">
          <span className="card-label">Permanently Reclaimed</span>
          <div className="storage-figure">
            {formatBytes(totals.permanentlyReclaimedBytes)}
          </div>
          <p className="stat-hint">
            Directly deleted or permanently cleared space.
          </p>
        </div>
      </div>

      {restoreMessage && (
        <div className="status-banner success" role="status">
          {restoreMessage}
        </div>
      )}
      {errorMessage && (
        <div className="status-banner error" role="alert">
          {errorMessage}
        </div>
      )}

      {selectedOperationId && operationDetail ? (
        <div className="operation-detail" data-testid="operation-detail">
          <div className="detail-header">
            <button
              type="button"
              className="back-button"
              onClick={handleBackToList}
              data-testid="back-to-operations"
            >
              &larr; Back to operations
            </button>
            <h3>
              Operation {operationDetail.operation.id} (
              {operationDetail.operation.actionMode === "permanentDelete"
                ? "Permanent Delete"
                : "Trash"}
              )
            </h3>
            <span className="detail-meta">
              {operationDetail.operation.totalItems.toString()} items total
              &bull; {operationDetail.operation.succeededItems.toString()}{" "}
              succeeded &bull;{" "}
              {operationDetail.operation.failedItems.toString()} failed
            </span>
          </div>

          <div className="items-table-wrapper">
            <table
              className="history-table"
              data-testid="operation-items-table"
            >
              <thead>
                <tr>
                  <th>Original Path</th>
                  <th>Size</th>
                  <th>Outcome</th>
                  <th>Restore Status & Action</th>
                </tr>
              </thead>
              <tbody>
                {operationDetail.items.map((item) => (
                  <tr
                    key={item.operationItemId}
                    data-testid={`item-${item.operationItemId}`}
                  >
                    <td className="path-cell" title={item.originalPath}>
                      <code>{item.originalPath}</code>
                    </td>
                    <td className="size-cell">{formatBytes(item.sizeBytes)}</td>
                    <td className="status-cell">
                      <span className={`status-tag status-${item.status}`}>
                        {item.status}
                      </span>
                      {item.errorMessage && (
                        <div className="error-text">{item.errorMessage}</div>
                      )}
                    </td>
                    <td className="action-cell">
                      {item.actionType === "permanentDelete" ? (
                        <span
                          className="status-tag permanent"
                          data-testid={`permanent-badge-${item.operationItemId}`}
                        >
                          Permanently deleted
                        </span>
                      ) : item.restoreEligibility.status === "restored" ? (
                        <span
                          className="status-tag restored"
                          data-testid={`restored-badge-${item.operationItemId}`}
                        >
                          Restored to {item.restoreEligibility.restored_to_path}
                        </span>
                      ) : item.restoreEligibility.status === "ineligible" ? (
                        <div
                          className="ineligible-box"
                          data-testid={`ineligible-reason-${item.operationItemId}`}
                        >
                          <span className="status-tag ineligible">
                            Ineligible
                          </span>
                          <span className="reason-text">
                            {item.restoreEligibility.reason}
                          </span>
                        </div>
                      ) : item.restoreEligibility.destination_occupied ? (
                        <div
                          className="collision-box"
                          data-testid={`destination-occupied-${item.operationItemId}`}
                        >
                          <span className="collision-warning">
                            Destination occupied
                          </span>
                          {item.restoreEligibility
                            .suggested_alternate_destination ? (
                            <button
                              type="button"
                              className="action-btn alt-restore"
                              data-testid={`restore-alt-btn-${item.operationItemId}`}
                              onClick={() =>
                                handleRestore(
                                  item.operationItemId,
                                  (
                                    item.restoreEligibility as {
                                      suggested_alternate_destination: string;
                                    }
                                  ).suggested_alternate_destination,
                                )
                              }
                            >
                              Restore as alternate
                            </button>
                          ) : (
                            <span className="collision-declined">
                              Destination occupied
                            </span>
                          )}
                        </div>
                      ) : (
                        <button
                          type="button"
                          className="action-btn primary"
                          data-testid={`restore-btn-${item.operationItemId}`}
                          onClick={() =>
                            handleRestore(item.operationItemId, null)
                          }
                        >
                          Restore
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : (
        <div className="operations-view" data-testid="operations-list">
          <div className="operations-filter-bar">
            <label htmlFor="outcome-filter">Filter by outcome: </label>
            <select
              id="outcome-filter"
              value={outcomeFilter}
              onChange={(e) => setOutcomeFilter(e.target.value)}
              className="filter-select"
              data-testid="outcome-filter-select"
            >
              <option value="all">All Outcomes</option>
              <option value="succeeded">Succeeded</option>
              <option value="failed">Failed / Permission Denied</option>
              <option value="skipped">Skipped (Protected)</option>
              <option value="blocked">Blocked / Vanished</option>
            </select>
          </div>

          {operations.length === 0 ? (
            <div className="empty-history" data-testid="empty-history">
              No operations recorded.
            </div>
          ) : (
            <div className="operations-grid">
              {operations.map((op) => (
                <div
                  key={op.id}
                  className="operation-card"
                  data-testid={`operation-card-${op.id}`}
                >
                  <div className="op-card-header">
                    <span className="op-mode-tag">
                      {op.actionMode === "permanentDelete"
                        ? "Permanent Delete"
                        : "Trash"}
                    </span>
                    <span className="op-date">
                      {new Date(Number(op.createdAtMs)).toLocaleString()}
                    </span>
                  </div>
                  <div className="op-metrics">
                    <div>
                      <strong>{op.totalItems.toString()}</strong> items total
                    </div>
                    <div>
                      {op.succeededItems > 0n && (
                        <span className="tag-succeeded">
                          {op.succeededItems.toString()} succeeded
                        </span>
                      )}
                      {op.failedItems > 0n && (
                        <span className="tag-failed">
                          {op.failedItems.toString()} failed
                        </span>
                      )}
                      {op.skippedItems > 0n && (
                        <span className="tag-skipped">
                          {op.skippedItems.toString()} skipped
                        </span>
                      )}
                      {op.blockedItems > 0n && (
                        <span className="tag-blocked">
                          {op.blockedItems.toString()} blocked
                        </span>
                      )}
                    </div>
                    <div className="op-bytes">
                      {op.actionMode === "trash"
                        ? `Pending in Trash: ${formatBytes(op.bytesPendingTrash)}`
                        : `Permanently Reclaimed: ${formatBytes(op.bytesPermanentlyReclaimed)}`}
                    </div>
                  </div>
                  <button
                    type="button"
                    className="inspect-op-btn"
                    data-testid={`inspect-op-${op.id}`}
                    onClick={() => handleSelectOperation(op.id)}
                  >
                    View Details
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
