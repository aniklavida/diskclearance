import { describe, expect, it } from "vitest";
import { fallbackFoundation } from "./foundation";

describe("foundation truthfulness", () => {
  it("does not claim scanning or deletion before implementation", () => {
    expect(fallbackFoundation.scanningImplemented).toBe(false);
    expect(fallbackFoundation.deletionImplemented).toBe(false);
  });
});
