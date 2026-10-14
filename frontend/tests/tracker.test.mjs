import assert from "node:assert/strict";
import { test } from "node:test";

import {
  createTracker,
  episodeDescription,
  requestUrl,
  watchedLabel,
} from "../static/scripts/tracker.mjs";

class FakeElement {
  constructor(tag = "div") {
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.attributes = {};
    this.listeners = new Map();
    this.hidden = false;
    this.disabled = false;
    this.value = "";
    this.textContent = "";
    this.className = "";
    this.classList = {
      values: new Set(),
      add: (name) => this.classList.values.add(name),
      contains: (name) => this.classList.values.has(name),
    };
  }

  addEventListener(name, callback) {
    const handlers = this.listeners.get(name) || [];
    handlers.push(callback);
    this.listeners.set(name, handlers);
  }

  setAttribute(name, value) {
    this.attributes[name] = String(value);
  }

  append(...children) {
    this.children.push(...children);
  }

  replaceChildren(...children) {
    this.children = [...children];
  }

  dispatch(name, event = {}) {
    for (const handler of this.listeners.get(name) || []) handler(event);
  }

  click() {
    this.dispatch("click", { preventDefault() {} });
  }
}

const ELEMENT_IDS = [
  "kind-filter",
  "start-date",
  "end-date",
  "clear-filters",
  "filter-error",
  "media-list",
  "media-loading",
  "empty-state",
  "error-state",
  "error-message",
  "retry-button",
  "result-count",
  "previous-page",
  "next-page",
  "page-indicator",
  "sync-status",
  "stats-loading",
  "theme-select",
  "activity-grid",
  "activity-loading",
  "activity-summary",
  "stat-movies",
  "stat-episodes",
  "stat-series",
  "stat-total",
  "stat-movies-recent",
  "stat-episodes-recent",
  "stat-series-recent",
  "filters",
];

class FakeDocument {
  constructor() {
    this.elements = new Map(ELEMENT_IDS.map((id) => [id, new FakeElement()]));
    this.documentElement = new FakeElement("html");
    this.getElementById("kind-filter").value = "all";
    this.getElementById("theme-select").value = "light";
    this.getElementById("media-loading").hidden = true;
    this.getElementById("stats-loading").hidden = true;
    this.getElementById("empty-state").hidden = true;
    this.getElementById("error-state").hidden = true;
    this.getElementById("filter-error").hidden = true;
    this.getElementById("previous-page").disabled = true;
    this.getElementById("next-page").disabled = true;
  }

  getElementById(id) {
    if (!this.elements.has(id)) this.elements.set(id, new FakeElement());
    return this.elements.get(id);
  }

  createElement(tag) {
    return new FakeElement(tag);
  }
}

function descendantElements(root) {
  return [root, ...root.children.flatMap(descendantElements)];
}

function response(body, ok = true, status = 200) {
  return { ok, status, async json() { return body; } };
}

function defaultActivity() {
  const first = new Date("2025-01-01T00:00:00Z");
  const days = Array.from({ length: 365 }, (_, index) => {
    const current = new Date(first);
    current.setUTCDate(first.getUTCDate() + index);
    return {
      date: current.toISOString().slice(0, 10),
      count: index < 5 ? [0, 1, 2, 4, 7][index] : 0,
    };
  });
  return { start_date: days[0].date, end_date: days.at(-1).date, days };
}

function defaultStats() {
  return {
    period: { start_date: null, end_date: null },
    movies: 1,
    tv_episodes: 2,
    tv_series: 1,
    total_items: 3,
    recent: {
      last_7_days: { movies: 1, tv_episodes: 1, tv_series: 1, total_items: 2 },
      last_30_days: { movies: 4, tv_episodes: 5, tv_series: 2, total_items: 9 },
    },
  };
}

function mediaItem(overrides = {}) {
  return {
    id: "00000000-0000-0000-0000-000000000001",
    kind: "movie",
    title: "A film",
    series_title: null,
    series_id: null,
    season_number: null,
    episode_number: null,
    watched_at: "2025-01-02T03:04:05.000000Z",
    play_count: 3,
    rewatch_count: 2,
    poster_url: "/api/posters/00000000-0000-0000-0000-000000000001.jpg",
    imdb_url: "https://www.imdb.com/title/tt1234567/",
    ...overrides,
  };
}

function mediaResponse(items = [mediaItem()], total = items.length, offset = 0) {
  return { items, total, limit: 50, offset };
}

function flush() {
  return new Promise((resolve) => setImmediate(resolve));
}

function createHarness(fetchImpl, options = {}) {
  const document = new FakeDocument();
  let intervalCallback = null;
  let intervalDelay = null;
  const tracker = createTracker({
    document,
    fetchImpl,
    origin: "https://tana.test",
    locale: "en-GB",
    setIntervalImpl(callback, delay) {
      intervalCallback = callback;
      intervalDelay = delay;
      return 1;
    },
    ...options,
  });
  return {
    document,
    tracker,
    get intervalCallback() { return intervalCallback; },
    get intervalDelay() { return intervalDelay; },
  };
}

function createDefaultFetch(media = mediaResponse()) {
  const calls = [];
  const status = {
    status: "success",
    last_started_at: "2025-01-02T04:00:00.000000Z",
    last_finished_at: "2025-01-02T04:01:00.000000Z",
    last_success_at: "2025-01-02T04:01:00.000000Z",
    items_seen: 1,
    items_imported: 1,
    last_error: null,
  };
  const handler = async (url) => {
    calls.push(url);
    if (url === "/api/config") return response({ timezone: "Europe/Berlin" });
    if (url === "/api/stats") return response(defaultStats());
    if (url === "/api/activity") return response(defaultActivity());
    if (url.startsWith("/api/media")) return response(media);
    if (url === "/api/sync/status") return response(status);
    throw new Error(`Unexpected URL ${url}`);
  };
  return { calls, handler, status };
}

test("initial load requests config, all-time stats, media defaults, and sync status", async () => {
  const fake = createDefaultFetch();
  const { document, tracker } = createHarness(fake.handler);
  await tracker.start();

  assert.deepEqual(new Set(fake.calls), new Set([
    "/api/config",
    "/api/stats",
    "/api/activity",
    "/api/media",
    "/api/sync/status",
  ]));
  assert.equal(document.getElementById("media-list").children.length, 1);
  assert.equal(document.getElementById("stat-movies").textContent, "1");
  assert.equal(document.documentElement.attributes["data-theme"], "dark");
  assert.equal(document.getElementById("theme-select").value, "dark");
  assert.equal(document.getElementById("activity-grid").children[0].children.length, 1);
  assert.equal(document.getElementById("activity-grid").children[0].children[0].children.length, 54);
  assert.equal(document.getElementById("activity-grid").children[1].children.length, 7);
  assert.equal(document.getElementById("activity-summary").textContent, "14 completed plays across 365 days");
  assert.match(document.getElementById("stat-movies-recent").textContent, /7 days: 1 · 30 days: 4/);
  assert.match(document.getElementById("sync-status").textContent, /Last synced/);
  assert.equal(document.getElementById("next-page").disabled, true);
});

test("activity heatmap includes zero days and applies fixed intensity levels", async () => {
  const fake = createDefaultFetch();
  const { document, tracker } = createHarness(fake.handler);
  await tracker.start();

  const table = descendantElements(document.getElementById("activity-grid"));
  const cells = table.filter((node) => node.tagName === "TD" && node.className.includes("activity-level"));
  assert.equal(cells.length, 365);
  assert.deepEqual(
    new Set(cells.map((cell) => cell.className)),
    new Set(Array.from({ length: 5 }, (_, level) => `activity-cell activity-level-${level}`)),
  );
  assert(cells.some((cell) => /0 completed plays/.test(cell.attributes["aria-label"])));
  assert(cells.some((cell) => /7 completed plays/.test(cell.attributes.title)));
});

test("saved theme is restored and theme changes persist", async () => {
  const fake = createDefaultFetch();
  const values = new Map([["tana-theme", "rose"]]);
  const storage = {
    getItem(key) { return values.get(key) ?? null; },
    setItem(key, value) { values.set(key, value); },
  };
  const { document, tracker } = createHarness(fake.handler, { storage });
  await tracker.start();

  assert.equal(document.documentElement.attributes["data-theme"], "rose");
  assert.equal(document.getElementById("theme-select").value, "rose");
  document.getElementById("theme-select").value = "dark";
  document.getElementById("theme-select").dispatch("change");
  assert.equal(document.documentElement.attributes["data-theme"], "dark");
  assert.equal(values.get("tana-theme"), "dark");
});

test("media cards safely render titles, posters, IMDb links, and rewatch labels", async () => {
  const hostileTitle = '<img src=x onerror="alert(1)">';
  const fake = createDefaultFetch(mediaResponse([mediaItem({ title: hostileTitle })]));
  const { document, tracker } = createHarness(fake.handler);
  await tracker.start();

  const nodes = descendantElements(document.getElementById("media-list"));
  const heading = nodes.find((node) => node.tagName === "H3");
  const image = nodes.find((node) => node.tagName === "IMG");
  const link = nodes.find((node) => node.tagName === "A");
  const playLabel = nodes.find((node) => node.className === "media-play-count");

  assert.equal(heading.textContent, hostileTitle);
  assert.equal(image.attributes.src, mediaItem().poster_url);
  assert.equal(image.attributes.alt, `Poster for ${hostileTitle}`);
  assert.equal(image.attributes.loading, "lazy");
  assert.equal(link.textContent, "IMDb");
  assert.equal(link.attributes.target, "_blank");
  assert.equal(link.attributes.rel, "noopener noreferrer");
  assert.equal(playLabel.textContent, "Watched 3 times · 2 rewatches");

  const placeholder = nodes.find((node) => node.className === "poster-placeholder");
  image.dispatch("error");
  assert.equal(image.hidden, true);
  assert.equal(placeholder.hidden, false);
});

test("TV details and play/rewatch labels are rendered accurately", async () => {
  const episode = mediaItem({
    kind: "tv",
    title: "Pilot",
    series_title: "Example series",
    season_number: 1,
    episode_number: 0,
    play_count: 1,
    rewatch_count: 0,
    poster_url: "/api/posters/00000000-0000-0000-0000-000000000005.jpg",
    imdb_url: null,
  });
  const rewatched = mediaItem({
    id: "00000000-0000-0000-0000-000000000002",
    title: "Rewatched film",
    play_count: 3,
    rewatch_count: 2,
    poster_url: null,
    imdb_url: null,
  });
  const fake = createDefaultFetch(mediaResponse([episode, rewatched]));
  const { document, tracker } = createHarness(fake.handler);
  await tracker.start();

  const nodes = descendantElements(document.getElementById("media-list"));
  assert(nodes.some((node) => node.textContent === "Example series"));
  assert(nodes.some((node) => node.textContent === "Season 1, Episode 0 — Pilot"));
  assert(nodes.some((node) => node.textContent === "Watched once"));
  assert(nodes.some((node) => node.textContent === "Watched 3 times · 2 rewatches"));
  const poster = nodes.find((node) => node.tagName === "IMG");
  assert.equal(poster.attributes.alt, "Poster for Example series");
});

test("invalid date range shows validation and makes no requests or result changes", async () => {
  const fake = createDefaultFetch();
  const { document, tracker } = createHarness(fake.handler);
  await tracker.start();
  const mediaList = document.getElementById("media-list");
  const oldCard = mediaList.children[0];
  const callsBefore = fake.calls.length;

  document.getElementById("start-date").value = "2025-03-10";
  document.getElementById("end-date").value = "2025-03-09";
  document.getElementById("end-date").dispatch("change");
  await flush();

  assert.equal(fake.calls.length, callsBefore);
  assert.equal(mediaList.children[0], oldCard);
  assert.equal(document.getElementById("filter-error").hidden, false);
  assert.match(document.getElementById("filter-error").textContent, /on or before/);
});

test("valid filters reset pagination and send the same dates to stats and media", async () => {
  const fake = createDefaultFetch();
  const { document, tracker } = createHarness(fake.handler);
  await tracker.start();
  fake.calls.length = 0;

  document.getElementById("kind-filter").value = "movie";
  document.getElementById("start-date").value = "2025-02-01";
  document.getElementById("end-date").value = "2025-02-28";
  document.getElementById("end-date").dispatch("change");
  await flush();

  assert.deepEqual(new Set(fake.calls), new Set([
    "/api/media?kind=movie&start_date=2025-02-01&end_date=2025-02-28",
    "/api/stats?start_date=2025-02-01&end_date=2025-02-28",
  ]));
  assert.equal(document.getElementById("stat-movies-recent").hidden, true);
  assert.equal(document.getElementById("filter-error").hidden, true);
});

test("clear filters resets controls and loads unfiltered data", async () => {
  const fake = createDefaultFetch();
  const { document, tracker } = createHarness(fake.handler);
  await tracker.start();
  document.getElementById("kind-filter").value = "tv";
  document.getElementById("start-date").value = "2025-01-01";
  document.getElementById("end-date").value = "2025-01-10";
  document.getElementById("end-date").dispatch("change");
  await flush();
  fake.calls.length = 0;

  document.getElementById("clear-filters").click();
  await flush();

  assert.equal(document.getElementById("kind-filter").value, "all");
  assert.equal(document.getElementById("start-date").value, "");
  assert.equal(document.getElementById("end-date").value, "");
  assert.deepEqual(new Set(fake.calls), new Set(["/api/media", "/api/stats"]));
});

test("pagination requests the next fixed-size page and renders result intervals", async () => {
  const firstPage = Array.from({ length: 50 }, (_, index) =>
    mediaItem({
      id: `00000000-0000-0000-0000-${String(index + 1).padStart(12, "0")}`,
      title: `Film ${index + 1}`,
      poster_url: null,
      imdb_url: null,
    }),
  );
  const calls = [];
  const handler = async (url) => {
    calls.push(url);
    if (url === "/api/config") return response({ timezone: "UTC" });
    if (url === "/api/stats") return response(defaultStats());
    if (url === "/api/sync/status") return response({ status: "never" });
    if (url === "/api/activity") return response(defaultActivity());
    if (url === "/api/media") return response(mediaResponse(firstPage, 60));
    if (url === "/api/media?offset=50") {
      const lastPage = Array.from({ length: 10 }, (_, index) =>
        mediaItem({
          id: `00000000-0000-0000-0000-${String(index + 51).padStart(12, "0")}`,
          title: `Film ${index + 51}`,
          poster_url: null,
          imdb_url: null,
        }),
      );
      return response(mediaResponse(lastPage, 60, 50));
    }
    throw new Error(`Unexpected URL ${url}`);
  };
  const { document, tracker } = createHarness(handler);
  await tracker.start();
  assert.equal(document.getElementById("next-page").disabled, false);
  document.getElementById("next-page").click();
  await flush();

  assert.equal(calls.at(-1), "/api/media?offset=50");
  assert.equal(document.getElementById("result-count").textContent, "51–60 of 60 results");
  assert.equal(document.getElementById("previous-page").disabled, false);
  assert.equal(document.getElementById("next-page").disabled, true);
  document.getElementById("previous-page").click();
  await flush();
  assert.equal(calls.at(-1), "/api/media");
});

test("empty results and failed request retries preserve prior results", async () => {
  let failMovieRequest = false;
  const calls = [];
  const handler = async (url) => {
    calls.push(url);
    if (url === "/api/config") return response({ timezone: "UTC" });
    if (url.startsWith("/api/stats")) return response(defaultStats());
    if (url === "/api/sync/status") return response({ status: "never" });
    if (url === "/api/activity") return response(defaultActivity());
    if (url === "/api/media") return response(mediaResponse([mediaItem()], 1));
    if (url === "/api/media?kind=movie") {
      if (failMovieRequest) {
        failMovieRequest = false;
        return response({}, false, 503);
      }
      return response(mediaResponse([mediaItem({ title: "Replacement" })], 1));
    }
    if (url === "/api/media?kind=tv") return response(mediaResponse([], 0));
    throw new Error(`Unexpected URL ${url}`);
  };
  const { document, tracker } = createHarness(handler);
  await tracker.start();
  const list = document.getElementById("media-list");
  const previousCard = list.children[0];
  failMovieRequest = true;
  document.getElementById("kind-filter").value = "movie";
  document.getElementById("kind-filter").dispatch("change");
  await flush();

  assert.equal(list.children[0], previousCard);
  assert.equal(document.getElementById("error-state").hidden, false);
  document.getElementById("retry-button").click();
  await flush();
  assert.equal(document.getElementById("error-state").hidden, true);
  assert.equal(list.children[0].children.at(-1).children[1].textContent, "Replacement");

  document.getElementById("kind-filter").value = "tv";
  document.getElementById("kind-filter").dispatch("change");
  await flush();
  assert.equal(document.getElementById("empty-state").hidden, false);
  assert.equal(document.getElementById("result-count").textContent, "0 results");
});

test("sync status polls every minute and uses the server timezone", async () => {
  const fake = createDefaultFetch();
  const harness = createHarness(fake.handler);
  await harness.tracker.start();
  assert.equal(harness.intervalDelay, 60_000);

  fake.status.status = "failed";
  fake.status.last_error = "Jellyfin request failed";
  await harness.intervalCallback();
  await flush();

  const status = harness.document.getElementById("sync-status").textContent;
  assert.match(status, /Last sync failed: Jellyfin request failed/);
  assert.match(status, /last successful sync/);
  assert.doesNotMatch(status, /UTC\)/);
});

test("pure rendering helpers handle play counts, episode numbers, and query values", () => {
  assert.equal(watchedLabel(0, 0), "Watched once");
  assert.equal(watchedLabel(1, 0), "Watched once");
  assert.equal(watchedLabel(5, 4), "Watched 5 times · 4 rewatches");
  assert.equal(episodeDescription({ season_number: null, episode_number: 3, title: "Pilot" }), "Episode 3 — Pilot");
  assert.equal(episodeDescription({ season_number: 1, episode_number: null, title: "Pilot" }), "Season 1 — Pilot");
  assert.equal(requestUrl("/api/media", { kind: "all", start_date: "", offset: 50 }), "/api/media?kind=all&offset=50");
});
