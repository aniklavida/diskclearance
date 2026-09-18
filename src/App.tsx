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

  useEffect(() => {
    invoke("foundation_status")
      .then((res: any) => setFoundation(res))
      .catch(() => setFoundation(fallbackFoundation));
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
        {activeTab === "Home" && <HomeTab />}
        {activeTab === "Cleanup" && <CleanupTab />}
        {activeTab === "Explore" && <ExploreTab />}
      </section>
    </main>
  );
}

function HomeTab() {
  return (
    <div data-testid="home-tab">
      <h1>See what goes. Keep what matters.</h1>
      <div>
        <div data-testid="reclaim-trash">Ready to move to Trash: 500 MB</div>
        <div data-testid="reclaim-freed">
          Space available after Trash is emptied: 750 MB
        </div>
      </div>
    </div>
  );
}

function CleanupTab() {
  return <div data-testid="cleanup-tab">Cleanup Content</div>;
}

function ExploreTab() {
  return <div data-testid="explore-tab">Explore Content</div>;
}
