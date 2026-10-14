const PAGE_SIZE = 50;
const DEFAULT_TIME_ZONE = "UTC";
const DEFAULT_THEME = "dark";
const THEMES = new Set(["light", "dark", "green", "rose"]);
const THEME_STORAGE_KEY = "tana-theme";

function element(document, tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function requestUrl(path, parameters = {}) {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(parameters)) {
    if (value !== undefined && value !== null && value !== "") {
      search.set(key, String(value));
    }
  }
  const query = search.toString();
  return query ? `${path}?${query}` : path;
}

function hasDateFilter(filters) {
  return Boolean(filters.start_date || filters.end_date);
}

function makeDateParameters(filters) {
  return {
    start_date: filters.start_date,
    end_date: filters.end_date,
  };
}

function watchedLabel(playCount, rewatchCount) {
  if (playCount <= 1) return "Watched once";
  const rewatches = Number.isInteger(rewatchCount)
    ? rewatchCount
    : Math.max(playCount - 1, 0);
  return `Watched ${playCount} times · ${rewatches} rewatches`;
}

function episodeDescription(item) {
  const parts = [];
  if (item.season_number !== null && item.season_number !== undefined) {
    parts.push(`Season ${item.season_number}`);
  }
  if (item.episode_number !== null && item.episode_number !== undefined) {
    parts.push(`Episode ${item.episode_number}`);
  }
  const numbering = parts.join(", ");
  if (!numbering) return item.title;
  return `${numbering} — ${item.title}`;
}

export function createTracker({
  document,
  fetchImpl = globalThis.fetch?.bind(globalThis),
  origin = globalThis.location?.origin || "http://localhost",
  setIntervalImpl = globalThis.setInterval?.bind(globalThis),
  storage = null,
  locale,
} = {}) {
  if (!document) throw new TypeError("A document implementation is required");
  if (typeof fetchImpl !== "function") throw new TypeError("A fetch implementation is required");

  const refs = {
    kind: document.getElementById("kind-filter"),
    start: document.getElementById("start-date"),
    end: document.getElementById("end-date"),
    clear: document.getElementById("clear-filters"),
    filterError: document.getElementById("filter-error"),
    mediaList: document.getElementById("media-list"),
    mediaLoading: document.getElementById("media-loading"),
    empty: document.getElementById("empty-state"),
    error: document.getElementById("error-state"),
    errorMessage: document.getElementById("error-message"),
    retry: document.getElementById("retry-button"),
    resultCount: document.getElementById("result-count"),
    previous: document.getElementById("previous-page"),
    next: document.getElementById("next-page"),
    pageIndicator: document.getElementById("page-indicator"),
    syncStatus: document.getElementById("sync-status"),
    statsLoading: document.getElementById("stats-loading"),
    theme: document.getElementById("theme-select"),
    activityTable: document.getElementById("activity-grid"),
    activityLoading: document.getElementById("activity-loading"),
    activitySummary: document.getElementById("activity-summary"),
    movies: document.getElementById("stat-movies"),
    episodes: document.getElementById("stat-episodes"),
    series: document.getElementById("stat-series"),
    total: document.getElementById("stat-total"),
    moviesRecent: document.getElementById("stat-movies-recent"),
    episodesRecent: document.getElementById("stat-episodes-recent"),
    seriesRecent: document.getElementById("stat-series-recent"),
  };

  for (const [name, reference] of Object.entries(refs)) {
    if (!reference) throw new Error(`Required frontend element is missing: ${name}`);
  }

  const state = {
    filters: { kind: "all", start_date: "", end_date: "" },
    offset: 0,
    timezone: DEFAULT_TIME_ZONE,
    media: null,
    stats: null,
    activity: null,
    syncStatus: null,
    theme: DEFAULT_THEME,
    failures: new Set(),
    mediaRequestId: 0,
    statsRequestId: 0,
    previousStatusText: "",
    pollHandle: null,
  };

  async function fetchJson(url) {
    const response = await fetchImpl(url, { headers: { Accept: "application/json" } });
    if (!response || !response.ok) {
      const status = response?.status ?? "network";
      throw new Error(`Request failed (${status})`);
    }
    return response.json();
  }

  function showError() {
    const hasFailures = state.failures.size > 0;
    refs.error.hidden = !hasFailures;
    if (hasFailures) {
      refs.errorMessage.textContent =
        "Some data could not be loaded. Please retry the failed request.";
    }
  }

  function formatDateTime(value) {
    try {
      return new Intl.DateTimeFormat(locale, {
        dateStyle: "medium",
        timeStyle: "short",
        timeZone: state.timezone,
      }).format(new Date(value));
    } catch {
      return value;
    }
  }

  function formatCount(value) {
    return new Intl.NumberFormat(locale).format(value ?? 0);
  }

  function renderStats() {
    if (!state.stats) return;
    const stats = state.stats;
    refs.movies.textContent = formatCount(stats.movies);
    refs.episodes.textContent = formatCount(stats.tv_episodes);
    refs.series.textContent = formatCount(stats.tv_series);
    refs.total.textContent = formatCount(stats.total_items);

    const filtered = hasDateFilter(state.filters);
    refs.moviesRecent.hidden = filtered;
    refs.episodesRecent.hidden = filtered;
    refs.seriesRecent.hidden = filtered;
    if (!filtered && stats.recent) {
      const week = stats.recent.last_7_days;
      const month = stats.recent.last_30_days;
      refs.moviesRecent.textContent = `7 days: ${formatCount(week.movies)} · 30 days: ${formatCount(month.movies)}`;
      refs.episodesRecent.textContent = `7 days: ${formatCount(week.tv_episodes)} · 30 days: ${formatCount(month.tv_episodes)}`;
      refs.seriesRecent.textContent = `7 days: ${formatCount(week.tv_series)} · 30 days: ${formatCount(month.tv_series)}`;
    } else {
      refs.moviesRecent.textContent = "";
      refs.episodesRecent.textContent = "";
      refs.seriesRecent.textContent = "";
    }
  }

  function sameOriginPosterUrl(value) {
    if (typeof value !== "string") return null;
    try {
      const url = new URL(value, origin);
      if (
        url.origin !== origin ||
        !/^\/api\/posters\/[0-9a-f-]+\.jpg$/i.test(url.pathname)
      ) {
        return null;
      }
      return `${url.pathname}${url.search}`;
    } catch {
      return null;
    }
  }

  function validImdbUrl(value) {
    return typeof value === "string" && /^https:\/\/www\.imdb\.com\/title\/tt[0-9]+\/$/.test(value)
      ? value
      : null;
  }

  function makePoster(item) {
    const posterUrl = sameOriginPosterUrl(item.poster_url);
    if (!posterUrl) return null;
    const frame = element(document, "figure", "poster-frame");
    const placeholder = element(document, "span", "poster-placeholder", "Poster unavailable");
    placeholder.hidden = true;
    const image = element(document, "img");
    image.setAttribute("src", posterUrl);
    const posterTitle = item.kind === "tv" ? item.series_title || item.title : item.title;
    image.setAttribute("alt", `Poster for ${posterTitle}`);
    image.setAttribute("loading", "lazy");
    image.setAttribute("width", "160");
    image.setAttribute("height", "240");
    image.addEventListener("error", () => {
      image.hidden = true;
      placeholder.hidden = false;
    });
    frame.append(image, placeholder);
    return frame;
  }

  function makeMediaCard(item) {
    const card = element(document, "article", "media-card");
    const poster = makePoster(item);
    if (poster) card.append(poster);
    else card.classList.add("media-card-no-poster");

    const content = element(document, "div", "media-card-content");
    const kind = element(
      document,
      "p",
      "media-kind",
      item.kind === "movie" ? "Movie" : "TV episode",
    );
    const title = element(
      document,
      "h3",
      "media-title",
      item.kind === "tv" ? item.series_title || item.title : item.title,
    );
    content.append(kind, title);

    if (item.kind === "tv") {
      const subtitle = element(document, "p", "media-subtitle", episodeDescription(item));
      content.append(subtitle);
    }

    const watched = element(
      document,
      "p",
      "media-watched",
      `Watched ${formatDateTime(item.watched_at)}`,
    );
    const plays = element(
      document,
      "p",
      "media-play-count",
      watchedLabel(item.play_count, item.rewatch_count),
    );
    content.append(watched, plays);

    if (item.kind === "movie") {
      const imdbUrl = validImdbUrl(item.imdb_url);
      if (imdbUrl) {
        const link = element(document, "a", "imdb-link", "IMDb");
        link.setAttribute("href", imdbUrl);
        link.setAttribute("target", "_blank");
        link.setAttribute("rel", "noopener noreferrer");
        content.append(link);
      }
    }

    card.append(content);
    return card;
  }

  function renderMedia() {
    const media = state.media;
    if (!media) return;

    refs.mediaList.replaceChildren();
    if (media.total === 0) {
      refs.empty.hidden = false;
      refs.resultCount.textContent = "0 results";
    } else {
      refs.empty.hidden = media.items.length > 0;
      if (media.items.length === 0) {
        refs.resultCount.textContent = `${formatCount(media.total)} results`;
      } else {
        const first = state.offset + 1;
        const last = state.offset + media.items.length;
        refs.resultCount.textContent = `${formatCount(first)}–${formatCount(last)} of ${formatCount(media.total)} results`;
        for (const item of media.items) refs.mediaList.append(makeMediaCard(item));
      }
    }

    const pageNumber = Math.floor(state.offset / PAGE_SIZE) + 1;
    const pageCount = media.total === 0 ? 0 : Math.ceil(media.total / PAGE_SIZE);
    refs.pageIndicator.textContent = pageCount ? `Page ${pageNumber} of ${pageCount}` : "Page 0 of 0";
    refs.previous.disabled = state.offset === 0;
    refs.next.disabled = state.offset + media.items.length >= media.total;
  }

  function activityLevel(count) {
    if (count === 0) return 0;
    if (count === 1) return 1;
    if (count <= 3) return 2;
    if (count <= 6) return 3;
    return 4;
  }

  function formatActivityDate(date) {
    try {
      return new Intl.DateTimeFormat(locale, {
        weekday: "long",
        month: "long",
        day: "numeric",
        timeZone: "UTC",
      }).format(new Date(`${date}T00:00:00Z`));
    } catch {
      return date;
    }
  }

  function renderActivity() {
    if (!state.activity) return;
    const days = state.activity.days;
    const firstDate = new Date(`${days[0].date}T00:00:00Z`);
    const firstRow = (firstDate.getUTCDay() + 6) % 7;
    const weekCount = Math.ceil((firstRow + days.length) / 7);
    const cells = Array.from({ length: 7 }, () => Array(weekCount).fill(null));
    const monthLabels = Array(weekCount).fill("");
    let previousMonth = "";
    for (let index = 0; index < days.length; index += 1) {
      const dayDate = new Date(`${days[index].date}T00:00:00Z`);
      const monthKey = days[index].date.slice(0, 7);
      if (monthKey !== previousMonth) {
        monthLabels[Math.floor((firstRow + index) / 7)] = new Intl.DateTimeFormat(
          locale,
          { month: "short", timeZone: "UTC" },
        ).format(dayDate);
        previousMonth = monthKey;
      }
      const position = firstRow + index;
      cells[position % 7][Math.floor(position / 7)] = days[index];
    }

    const tableHead = element(document, "thead");
    const monthRow = element(document, "tr");
    monthRow.append(element(document, "th", "activity-day-label"));
    for (const month of monthLabels) {
      const monthCell = element(document, "th", "activity-month-label", month);
      monthCell.setAttribute("scope", "col");
      monthRow.append(monthCell);
    }
    tableHead.append(monthRow);

    const tableBody = element(document, "tbody");
    const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    for (let rowIndex = 0; rowIndex < 7; rowIndex += 1) {
      const row = element(document, "tr");
      const dayHeader = element(document, "th", "activity-day-label", weekdays[rowIndex]);
      dayHeader.setAttribute("scope", "row");
      row.append(dayHeader);
      for (const day of cells[rowIndex]) {
        const cell = element(document, "td");
        if (day === null) {
          cell.setAttribute("aria-hidden", "true");
          row.append(cell);
          continue;
        }
        const level = activityLevel(day.count);
        const label = `${formatActivityDate(day.date)}: ${day.count} completed ${day.count === 1 ? "play" : "plays"}`;
        cell.className = `activity-cell activity-level-${level}`;
        cell.setAttribute("title", label);
        cell.setAttribute("aria-label", label);
        row.append(cell);
      }
      tableBody.append(row);
    }
    refs.activityTable.replaceChildren(tableHead, tableBody);
    const total = days.reduce((sum, day) => sum + day.count, 0);
    refs.activitySummary.textContent = `${formatCount(total)} completed plays across ${days.length} days`;
  }

  function applyTheme(theme, persist = true) {
    const selected = THEMES.has(theme) ? theme : "light";
    state.theme = selected;
    refs.theme.value = selected;
    document.documentElement.setAttribute("data-theme", selected);
    if (persist && storage) {
      try {
        storage.setItem(THEME_STORAGE_KEY, selected);
      } catch {
        // Theme remains active for this page session if browser storage is unavailable.
      }
    }
  }

  function restoreTheme() {
    let savedTheme = DEFAULT_THEME;
    if (storage) {
      try {
        savedTheme = storage.getItem(THEME_STORAGE_KEY) || DEFAULT_THEME;
      } catch {
        savedTheme = DEFAULT_THEME;
      }
    }
    applyTheme(savedTheme, false);
  }

  function renderSyncStatus() {
    if (!state.syncStatus) return;
    const status = state.syncStatus;
    let message;
    if (status.status === "never") {
      message = "Never synced";
    } else if (status.status === "running") {
      message = status.last_success_at
        ? `Sync in progress · last successful sync ${formatDateTime(status.last_success_at)}`
        : "Sync in progress · no successful sync yet";
    } else if (status.status === "failed") {
      message = `Last sync failed${status.last_error ? `: ${status.last_error}` : ""}`;
      if (status.last_success_at) {
        message += ` · last successful sync ${formatDateTime(status.last_success_at)}`;
      }
    } else {
      message = status.last_success_at
        ? `Last synced ${formatDateTime(status.last_success_at)}`
        : "Never synced";
    }
    refs.syncStatus.textContent = message;
    if (message !== state.previousStatusText) {
      state.previousStatusText = message;
    }
  }

  function renderAll() {
    renderStats();
    renderActivity();
    renderMedia();
    renderSyncStatus();
    showError();
  }

  function mediaUrl() {
    const parameters = {
      kind: state.filters.kind === "all" ? "" : state.filters.kind,
      start_date: state.filters.start_date,
      end_date: state.filters.end_date,
      offset: state.offset > 0 ? state.offset : "",
    };
    return requestUrl("/api/media", parameters);
  }

  function statsUrl() {
    return requestUrl("/api/stats", makeDateParameters(state.filters));
  }

  async function loadMedia() {
    const requestId = ++state.mediaRequestId;
    let loaded = false;
    refs.mediaLoading.hidden = false;
    try {
      const data = await fetchJson(mediaUrl());
      if (requestId !== state.mediaRequestId) return;
      state.media = data;
      state.failures.delete("media");
      loaded = true;
    } catch {
      if (requestId === state.mediaRequestId) state.failures.add("media");
    } finally {
      if (requestId === state.mediaRequestId) {
        refs.mediaLoading.hidden = true;
        if (loaded) renderMedia();
        else if (!state.media) refs.resultCount.textContent = "Results unavailable";
        showError();
      }
    }
  }

  async function loadStats() {
    const requestId = ++state.statsRequestId;
    let loaded = false;
    refs.statsLoading.hidden = false;
    try {
      const data = await fetchJson(statsUrl());
      if (requestId !== state.statsRequestId) return;
      state.stats = data;
      state.failures.delete("stats");
      loaded = true;
    } catch {
      if (requestId === state.statsRequestId) state.failures.add("stats");
    } finally {
      if (requestId === state.statsRequestId) {
        refs.statsLoading.hidden = true;
        if (loaded) renderStats();
        else if (!state.stats) {
          refs.movies.textContent = "—";
          refs.episodes.textContent = "—";
          refs.series.textContent = "—";
          refs.total.textContent = "—";
        }
        showError();
      }
    }
  }

  async function loadConfig() {
    let loaded = false;
    try {
      const data = await fetchJson("/api/config");
      if (typeof data.timezone !== "string" || !data.timezone) {
        throw new Error("Configuration response was invalid");
      }
      state.timezone = data.timezone;
      state.failures.delete("config");
      loaded = true;
    } catch {
      state.failures.add("config");
    }
    if (loaded) renderAll();
    else showError();
  }

  async function loadActivity() {
    refs.activityLoading.hidden = false;
    try {
      state.activity = await fetchJson("/api/activity");
      state.failures.delete("activity");
      renderActivity();
    } catch {
      state.failures.add("activity");
    } finally {
      refs.activityLoading.hidden = true;
      showError();
    }
  }

  async function loadSyncStatus() {
    try {
      state.syncStatus = await fetchJson("/api/sync/status");
      state.failures.delete("status");
    } catch {
      state.failures.add("status");
      if (!state.syncStatus) refs.syncStatus.textContent = "Sync status unavailable";
    }
    renderSyncStatus();
    showError();
  }

  async function retryFailed() {
    const pending = [...state.failures];
    const retries = [];
    if (pending.includes("config")) retries.push(loadConfig());
    if (pending.includes("media")) retries.push(loadMedia());
    if (pending.includes("stats")) retries.push(loadStats());
    if (pending.includes("activity")) retries.push(loadActivity());
    if (pending.includes("status")) retries.push(loadSyncStatus());
    await Promise.all(retries);
  }

  function currentFilters() {
    return {
      kind: refs.kind.value || "all",
      start_date: refs.start.value || "",
      end_date: refs.end.value || "",
    };
  }

  function applyFilters() {
    const filters = currentFilters();
    if (filters.start_date && filters.end_date && filters.start_date > filters.end_date) {
      refs.filterError.textContent = "The from date must be on or before the through date.";
      refs.filterError.hidden = false;
      return;
    }
    state.filters = filters;
    state.offset = 0;
    refs.filterError.textContent = "";
    refs.filterError.hidden = true;
    renderStats();
    void Promise.all([loadMedia(), loadStats()]);
  }

  function clearFilters() {
    refs.kind.value = "all";
    refs.start.value = "";
    refs.end.value = "";
    state.filters = { kind: "all", start_date: "", end_date: "" };
    state.offset = 0;
    refs.filterError.textContent = "";
    refs.filterError.hidden = true;
    void Promise.all([loadMedia(), loadStats()]);
  }

  function previousPage() {
    if (state.offset === 0) return;
    state.offset = Math.max(0, state.offset - PAGE_SIZE);
    void loadMedia();
  }

  function nextPage() {
    if (!state.media || state.offset + state.media.items.length >= state.media.total) return;
    state.offset += PAGE_SIZE;
    void loadMedia();
  }

  function bindEvents() {
    refs.kind.addEventListener("change", applyFilters);
    refs.start.addEventListener("change", applyFilters);
    refs.end.addEventListener("change", applyFilters);
    refs.clear.addEventListener("click", clearFilters);
    refs.theme.addEventListener("change", () => applyTheme(refs.theme.value));
    refs.retry.addEventListener("click", () => void retryFailed());
    refs.previous.addEventListener("click", previousPage);
    refs.next.addEventListener("click", nextPage);
    document.getElementById("filters").addEventListener("submit", (event) => {
      event.preventDefault();
      applyFilters();
    });
  }

  async function start() {
    restoreTheme();
    bindEvents();
    await Promise.all([
      loadConfig(),
      loadStats(),
      loadActivity(),
      loadMedia(),
      loadSyncStatus(),
    ]);
    if (typeof setIntervalImpl === "function") {
      state.pollHandle = setIntervalImpl(() => void loadSyncStatus(), 60_000);
    }
    return state;
  }

  return {
    start,
    state,
    retryFailed,
    applyFilters,
    clearFilters,
    loadMedia,
    loadStats,
    loadSyncStatus,
  };
}

export { episodeDescription, requestUrl, watchedLabel };
