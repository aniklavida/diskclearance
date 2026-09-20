import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  invokeFetchHistoryOperations,
  invokeFetchLifetimeReclamationTotals,
  invokeFetchOperationDetail,
  invokeRestoreItem,
} from "./boundary/client";
import type {
  HistoryOperationDetail,
  HistoryOperationSummary,
  LifetimeReclamationTotals,
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

  return (
    <main className="window-shell">
      <aside className="sidebar" aria-label="Primary navigation">
        <div className="brand-mark" aria-hidden="true">
          <span />
        </div>
        <strong>DiskClearance</strong>
        <nav>
          {["Home", "Cleanup", "Explore", "Applications", "History"].map(
            (item) => (
              <button
                key={item}
                className={`nav-item ${activeTab === item ? "active" : ""}`}
                type="button"
                disabled={item === "Applications"}
                onClick={() => setActiveTab(item)}
              >
                {item}
                {item === "Applications" && <small>Planned</small>}
              </button>
            ),
          )}
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
}: {
  report: { pendingInTrashBytes: number; permanentlyReclaimedBytes: number };
}) {
  return (
    <div data-testid="home-tab">
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
    </div>
  );
}

function CleanupTab() {
  const [findings, setFindings] = useState<any[]>([]);
  const [selected, setSelected] = useState<Record<string, boolean>>({});

  useEffect(() => {
    invoke("fetch_findings_page", { sessionId: "default", limit: 50 })
      .then((res: any) => setFindings(res.items || []))
      .catch(() => {});
  }, []);

  const toggleSelect = (id: string) => {
    setSelected((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  return (
    <div data-testid="cleanup-tab">
      <h2>Actionable Findings</h2>
      <ul>
        {findings.map((f) => (
          <li key={f.id}>
            <input
              type="checkbox"
              checked={!!selected[f.id]}
              onChange={() => toggleSelect(f.id)}
            />
            {f.path} - {f.sizeBytes} bytes ({f.actionKind})
          </li>
        ))}
      </ul>
    </div>
  );
}

function ExploreTab() {
  const [agg, setAgg] = useState<any>(null);

  useEffect(() => {
    invoke("fetch_folder_aggregate", { sessionId: "default", path: "/" })
      .then((res: any) => setAgg(res))
      .catch(() => {});
  }, []);

  return (
    <div data-testid="explore-tab">
      <h2>Explore Folders</h2>
      {agg ? (
        <div>
          <h3>
            Path: {agg.path} (Total: {agg.totalSizeBytes} bytes, Files:{" "}
            {agg.fileCount})
          </h3>
          <ul>
            {agg.children &&
              agg.children.map((c: any) => (
                <li key={c.name}>
                  {c.isDir ? "[DIR]" : "[FILE]"} {c.name} - {c.sizeBytes} bytes
                </li>
              ))}
          </ul>
        </div>
      ) : (
        <p>No aggregation data or error fetching.</p>
      )}
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
