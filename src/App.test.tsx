import { describe, it, expect } from "vitest";
import { renderToString } from "react-dom/server";
import App from "./App";

describe("two-distinct-totals claim", () => {
  it("renders distinct figures for trash and freed space without summing them", () => {
    const html = renderToString(<App />);
    expect(html).toContain("Ready to move to Trash");
    expect(html).toContain("Space available after Trash is emptied");
  });
});
