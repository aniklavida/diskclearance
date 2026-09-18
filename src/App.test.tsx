import { describe, it, expect } from "vitest";
import { renderToString } from "react-dom/server";
import { HomeTab } from "./App";

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
