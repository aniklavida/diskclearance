import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { fallbackFoundation } from "./foundation";
import type { FoundationStatus } from "./types/bindings";

export default function App() {
  const [foundation, setFoundation] = useState(fallbackFoundation);

  useEffect(() => {
    invoke<FoundationStatus>("foundation_status")
      .then(setFoundation)
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
          <button className="nav-item active" type="button">
            Home
          </button>
          {["Cleanup", "Explore", "Applications", "History"].map((item) => (
            <button className="nav-item" disabled key={item} type="button">
              {item}
              <small>Planned</small>
            </button>
          ))}
        </nav>
      </aside>

      <section className="content">
        <div className="status-pill">Foundation</div>
        <p className="eyebrow">{foundation.status}</p>
        <h1>
          See what goes.
          <br />
          Keep what matters.
        </h1>
        <p className="lede">
          The safe storage workflow is designed. Scanning and deletion remain
          disabled until their safety tests are implemented.
        </p>

        <div className="safety-card">
          <div>
            <span className="card-label">Current capability</span>
            <strong>Application foundation</strong>
          </div>
          <dl>
            <div>
              <dt>Scanning</dt>
              <dd>
                {foundation.scanningImplemented
                  ? "Available"
                  : "Planned for v1.0"}
              </dd>
            </div>
            <div>
              <dt>Deletion</dt>
              <dd>
                {foundation.deletionImplemented ? "Available" : "Disabled"}
              </dd>
            </div>
          </dl>
        </div>
      </section>
    </main>
  );
}
