import { describe, expect, it } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";

export function sRGBToLinear(channel: number): number {
  const c = channel / 255;
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

export function hexToRgb(hex: string): [number, number, number] {
  let clean = hex.replace("#", "").trim();
  if (clean.length === 3) {
    clean = clean
      .split("")
      .map((c) => c + c)
      .join("");
  }
  const val = parseInt(clean, 16);
  return [(val >> 16) & 255, (val >> 8) & 255, val & 255];
}

export function relativeLuminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex);
  return (
    0.2126 * sRGBToLinear(r) +
    0.7152 * sRGBToLinear(g) +
    0.0722 * sRGBToLinear(b)
  );
}

export function contrastRatio(color1: string, color2: string): number {
  const l1 = relativeLuminance(color1);
  const l2 = relativeLuminance(color2);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

export interface ParsedTokens {
  light: Record<string, string>;
  dark: Record<string, string>;
}

export function parseCssTokens(css: string): ParsedTokens {
  const lightMatch = css.match(/:root\s*\{([^}]+)\}/);
  const darkMatch = css.match(
    /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{\s*:root\s*\{([^}]+)\}/,
  );

  function extractVars(block: string): Record<string, string> {
    const vars: Record<string, string> = {};
    const regex = /(--[\w-]+)\s*:\s*([^;]+);/g;
    let m: RegExpExecArray | null;
    while ((m = regex.exec(block)) !== null) {
      vars[m[1].trim()] = m[2].trim();
    }
    return vars;
  }

  const light = lightMatch ? extractVars(lightMatch[1]) : {};
  const dark = darkMatch ? extractVars(darkMatch[1]) : {};

  // Resolve vars in tokens
  // A token may point at another token. Following that chain needs two
  // guards, and the original had neither: an unresolvable name assigned
  // `current` back to itself, and a cycle between two tokens would do the
  // same — either one spins forever rather than failing. A test suite that
  // hangs is worse than one that fails, because it reads as a slow machine.
  function resolve(
    val: string,
    tokens: Record<string, string>,
    fallback: Record<string, string>,
  ): string {
    let current = val;
    const seen = new Set<string>();

    // Only a value that is *entirely* one reference is followed. A composite
    // such as `var(--focus-ring-width) solid var(--focus-ring-color)` is not a
    // colour and has nothing to resolve to; stripping its outer `var(` and `)`
    // produced a nonsense name and sent the old loop looking for it.
    let ref = /^var\(\s*(--[\w-]+)\s*\)$/.exec(current);
    while (ref) {
      const varName = ref[1];

      if (seen.has(varName)) {
        throw new Error(
          `token reference cycle: ${[...seen, varName].join(" -> ")}`,
        );
      }
      seen.add(varName);

      const next = tokens[varName] ?? fallback[varName];
      if (next === undefined) {
        throw new Error(
          `token ${varName} is referenced but never defined; ` +
            `a colour that resolves to nothing cannot be contrast-checked`,
        );
      }
      current = next;
      ref = /^var\(\s*(--[\w-]+)\s*\)$/.exec(current);
    }
    return current;
  }

  for (const k of Object.keys(light)) {
    light[k] = resolve(light[k], light, {});
  }
  for (const k of Object.keys(dark)) {
    dark[k] = resolve(dark[k], dark, light);
  }

  return { light, dark };
}

describe("WCAG contrast calculation reference checks", () => {
  it("calculates exactly 21:1 for white on black", () => {
    const ratio = contrastRatio("#ffffff", "#000000");
    expect(ratio).toBeCloseTo(21.0, 1);
  });

  it("calculates published ~4.54:1 for #767676 on white", () => {
    const ratio = contrastRatio("#767676", "#ffffff");
    expect(ratio).toBeGreaterThanOrEqual(4.5);
    expect(ratio).toBeCloseTo(4.54, 1);
  });
});

describe("Design tokens and contrast validation", () => {
  const cssPath = path.resolve(__dirname, "App.css");
  const cssContent = fs.readFileSync(cssPath, "utf-8");
  const tokens = parseCssTokens(cssContent);

  it("does not allow literal hex colors outside token definition blocks in App.css", () => {
    // Remove :root blocks and dark media query block
    const cleaned = cssContent
      .replace(/:root\s*\{[^}]+\}/g, "")
      .replace(/@media\s*\(prefers-color-scheme:\s*dark\)\s*\{[^}]+\}/g, "");

    const hexMatches = cleaned.match(/#[0-9a-fA-F]{3,6}/g);
    expect(hexMatches).toBeNull();
  });

  it("defines spacing scale tokens 4, 8, 12, 16, 24, 32, 48", () => {
    const expected = ["4px", "8px", "12px", "16px", "24px", "32px", "48px"];
    expected.forEach((val) => {
      const num = parseInt(val, 10);
      expect(tokens.light[`--space-${num}`]).toBe(val);
    });
  });

  it("defines radius scale tokens 8 (control), 12 (card), 16 (surface)", () => {
    expect(tokens.light["--radius-control"]).toBe("8px");
    expect(tokens.light["--radius-card"]).toBe("12px");
    expect(tokens.light["--radius-surface"]).toBe("16px");
  });

  it("defines target size and focus ring tokens", () => {
    expect(tokens.light["--target-min"]).toBe("32px");
    expect(tokens.light["--target-primary"]).toBe("40px");
    expect(tokens.light["--focus-ring-width"]).toBe("2px");
    expect(tokens.light["--focus-ring-offset"]).toBe("2px");
  });

  it("fixes the .status-pill dark contrast failure and asserts AA on that specific pair", () => {
    // Light status-pill pair: --accent-fg on --accent-soft
    const lightFg = tokens.light["--accent-fg"];
    const lightBg = tokens.light["--accent-soft"];
    expect(lightFg).toBeDefined();
    expect(lightBg).toBeDefined();
    const lightRatio = contrastRatio(lightFg, lightBg);
    expect(lightRatio).toBeGreaterThanOrEqual(4.5);

    // Dark status-pill pair: --accent-fg on --accent-soft
    const darkFg = tokens.dark["--accent-fg"];
    const darkBg = tokens.dark["--accent-soft"];
    expect(darkFg).toBeDefined();
    expect(darkBg).toBeDefined();
    const darkRatio = contrastRatio(darkFg, darkBg);
    expect(darkRatio).toBeGreaterThanOrEqual(4.5);

    // Document that the old hardcoded pair #315d51 on #263d36 failed (< 1.6:1)
    const oldDarkFailureRatio = contrastRatio("#315d51", "#263d36");
    expect(oldDarkFailureRatio).toBeLessThan(2.0);
  });

  it("asserts primary and secondary text tokens meet AA contrast on all surfaces", () => {
    // Light appearance
    expect(
      contrastRatio(tokens.light["--text-primary"], tokens.light["--window"]),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio(tokens.light["--text-primary"], tokens.light["--surface"]),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio(
        tokens.light["--text-primary"],
        tokens.light["--surface-raised"],
      ),
    ).toBeGreaterThanOrEqual(4.5);

    expect(
      contrastRatio(tokens.light["--text-secondary"], tokens.light["--window"]),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio(
        tokens.light["--text-secondary"],
        tokens.light["--surface"],
      ),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio(
        tokens.light["--text-secondary"],
        tokens.light["--surface-raised"],
      ),
    ).toBeGreaterThanOrEqual(4.5);

    // Dark appearance
    expect(
      contrastRatio(tokens.dark["--text-primary"], tokens.dark["--window"]),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio(tokens.dark["--text-primary"], tokens.dark["--surface"]),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio(
        tokens.dark["--text-primary"],
        tokens.dark["--surface-raised"],
      ),
    ).toBeGreaterThanOrEqual(4.5);

    expect(
      contrastRatio(tokens.dark["--text-secondary"], tokens.dark["--window"]),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio(tokens.dark["--text-secondary"], tokens.dark["--surface"]),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio(
        tokens.dark["--text-secondary"],
        tokens.dark["--surface-raised"],
      ),
    ).toBeGreaterThanOrEqual(4.5);
  });

  it("asserts Rebuildable class tokens meet AA in both appearances", () => {
    // Light
    const lightRatio = contrastRatio(
      tokens.light["--class-rebuildable-fg"],
      tokens.light["--class-rebuildable-bg"],
    );
    expect(lightRatio).toBeGreaterThanOrEqual(4.5);

    // Dark
    const darkRatio = contrastRatio(
      tokens.dark["--class-rebuildable-fg"],
      tokens.dark["--class-rebuildable-bg"],
    );
    expect(darkRatio).toBeGreaterThanOrEqual(4.5);
  });

  it("asserts Review class tokens meet AA in both appearances", () => {
    // Light
    const lightRatio = contrastRatio(
      tokens.light["--class-review-fg"],
      tokens.light["--class-review-bg"],
    );
    expect(lightRatio).toBeGreaterThanOrEqual(4.5);

    // Dark
    const darkRatio = contrastRatio(
      tokens.dark["--class-review-fg"],
      tokens.dark["--class-review-bg"],
    );
    expect(darkRatio).toBeGreaterThanOrEqual(4.5);
  });

  it("asserts Protected class tokens meet AA in both appearances", () => {
    // Light
    const lightRatio = contrastRatio(
      tokens.light["--class-protected-fg"],
      tokens.light["--class-protected-bg"],
    );
    expect(lightRatio).toBeGreaterThanOrEqual(4.5);

    // Dark
    const darkRatio = contrastRatio(
      tokens.dark["--class-protected-fg"],
      tokens.dark["--class-protected-bg"],
    );
    expect(darkRatio).toBeGreaterThanOrEqual(4.5);
  });

  it("asserts irreversible/error class tokens meet AA in both appearances", () => {
    // Light
    const lightRatio = contrastRatio(
      tokens.light["--class-irreversible-fg"],
      tokens.light["--class-irreversible-bg"],
    );
    expect(lightRatio).toBeGreaterThanOrEqual(4.5);

    // Dark
    const darkRatio = contrastRatio(
      tokens.dark["--class-irreversible-fg"],
      tokens.dark["--class-irreversible-bg"],
    );
    expect(darkRatio).toBeGreaterThanOrEqual(4.5);
  });
});
