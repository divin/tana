import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

const publicDir = new URL("../public/", import.meta.url);

test("Zola generates the accessible Tana page, module script, and Sass stylesheet", async () => {
  const [html, css, script] = await Promise.all([
    readFile(new URL("index.html", publicDir), "utf8"),
    readFile(new URL("main.css", publicDir), "utf8"),
    readFile(new URL("scripts/main.mjs", publicDir), "utf8"),
  ]);

  assert.match(html, /<html lang="en" data-theme="dark">/);
  assert.match(html, /<title>Tana — Watched media<\/title>/);
  assert.match(html, /<h1>Watched media<\/h1>/);
  assert.match(html, /href="\/main\.css"/);
  assert.match(html, /type="module" src="\/scripts\/main\.mjs"/);
  assert.match(html, /id="kind-filter"/);
  assert.match(html, /id="activity-grid"/);
  assert.match(html, /id="theme-select"/);
  assert.match(html, /<option value="rose">Rose<\/option>/);
  assert.match(html, /aria-live="polite"/);
  assert.match(css, /system-ui/);
  assert.match(css, /media-card-no-poster/);
  assert.match(css, /\.activity-grid\{width:100%;min-width:900px;table-layout:fixed/);
  assert.match(css, /\.filter-field input,.filter-field select\{display:block;box-sizing:border-box;width:100%;min-width:0;max-width:100%/);
  assert.match(css, /\.filter-field\{display:grid;width:100%;min-width:0/);
  assert.match(css, /@supports \(-webkit-touch-callout: none\)/);
  assert.match(css, /@media \(max-width: 480px\)/);
  assert.match(css, /html\[data-theme=dark\]/);
  assert.match(css, /html\[data-theme=green\]/);
  assert.match(css, /html\[data-theme=rose\]/);
  assert.match(css, /\.activity-legend \.activity-cell\{display:block\}/);
  assert.doesNotMatch(css, /td\.activity-cell\{[^}]*display:block/);
  assert.match(script, /createTracker/);
});
