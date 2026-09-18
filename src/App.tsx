import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// Basic fallback
const fallbackFoundation = {
  status: "Offline",
  scanningImplemented: false,
  deletionImplemented: false,
};

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
                disabled={item === "Applications" || item === "History"}
                onClick={() => setActiveTab(item)}
              >
                {item}
                {(item === "Applications" || item === "History") && (
                  <small>Planned</small>
                )}
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
