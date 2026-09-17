import { describe, expect, it } from "vitest";
// Read from disk rather than importing: vitest stubs CSS imports, so
// `./App.css?raw` resolves to an empty string and every assertion below would
// silently pass against nothing. Same reasoning as `tokens.test.ts`.
//
// @ts-expect-error type error without @types/node package
import { readFileSync, readdirSync } from "node:fs";

/**
 * The design documents quote concrete hex values beside the token names they
 * belong to — `--class-review-bg` (`#faecc6`), and token tables with a Light
 * and a Dark column. Those values are copied by hand and nothing has stopped
 * them drifting from the stylesheet they claim to describe.
 *
 * A document that names the wrong colour is worse than one that names none: it
 * reads as authoritative and is quietly false. This checks every such claim
 * against `src/App.css`.
 */

type Palette = { light: Map<string, string>; dark: Map<string, string> };

const DARK_AT = "@media (prefers-color-scheme: dark)";

function readTokens(block: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of block.matchAll(/(--[\w-]+)\s*:\s*(#[0-9a-fA-F]{3,8})/g)) {
    out.set(m[1], m[2].toLowerCase());
  }
  return out;
}

function palette(): Palette {
  const css = readFileSync("src/App.css", "utf-8") as string;
  const at = css.indexOf(DARK_AT);
  expect(
    at,
    `App.css must contain "${DARK_AT}" for a dark palette to exist`,
  ).toBeGreaterThan(-1);
  return {
    light: readTokens(css.slice(0, at)),
    dark: readTokens(css.slice(at)),
  };
}

function designDocs(): { name: string; text: string }[] {
  const dir = "docs/design";
  return (readdirSync(dir) as string[])
    .filter((f: string) => f.endsWith(".md"))
    .map((f: string) => ({
      name: `${dir}/${f}`,
      text: readFileSync(`${dir}/${f}`, "utf-8") as string,
    }));
}

describe("design documents describe the stylesheet that exists", () => {
  const { light, dark } = palette();

  it("has a palette to check against", () => {
    expect(light.size).toBeGreaterThan(0);
    expect(dark.size).toBeGreaterThan(0);
  });

  it("quotes no hex value that disagrees with App.css", () => {
    const wrong: string[] = [];
    let checked = 0;

    for (const { name, text } of designDocs()) {
      // Inline form: `--token` (`#hex`)
      for (const m of text.matchAll(
        /`(--[\w-]+)`\s*\(`(#[0-9a-fA-F]{3,8})`/g,
      )) {
        const [, token, hex] = m;
        const l = light.get(token);
        const d = dark.get(token);
        if (l === undefined && d === undefined) {
          wrong.push(`${name}: ${token} is not defined in App.css at all`);
          continue;
        }
        checked += 1;
        const v = hex.toLowerCase();
        if (v !== l && v !== d) {
          wrong.push(
            `${name}: ${token} quoted as ${hex}, App.css has light=${l} dark=${d}`,
          );
        }
      }

      // Table form: | `--token` | #light | #dark |
      for (const m of text.matchAll(
        /\|\s*`?(--[\w-]+)`?\s*\|\s*`?(#[0-9a-fA-F]{3,8})`?\s*\|\s*`?(#[0-9a-fA-F]{3,8})`?\s*\|/g,
      )) {
        const [, token, lHex, dHex] = m;
        checked += 1;
        if (light.get(token) !== lHex.toLowerCase()) {
          wrong.push(
            `${name}: ${token} light column says ${lHex}, App.css has ${light.get(token)}`,
          );
        }
        if (dark.get(token) !== dHex.toLowerCase()) {
          wrong.push(
            `${name}: ${token} dark column says ${dHex}, App.css has ${dark.get(token)}`,
          );
        }
      }
    }

    // Guards the guard: if the extraction silently stops matching, this fails
    // rather than reporting a vacuous pass.
    expect(
      checked,
      "no token/hex claims were found in docs/design — the extraction is broken, not the documents",
    ).toBeGreaterThan(20);
    expect(wrong, wrong.join("\n")).toEqual([]);
  });
});
