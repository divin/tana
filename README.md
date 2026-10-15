# Tana 🎬

**Tana (棚, たな, “shelf” in Japanese) is a privacy-friendly, self-hosted watch-history tracker for Jellyfin.** It records completed movie and episode plays server-side from Jellyfin's Playback Stop webhook, stores play counts and timestamps in a local SQLite database, and serves a small web UI with statistics, filters, and poster images. All data stays on your own host; the only outbound traffic is to your Jellyfin server for metadata and posters.

AI tools were used during development.

## 🧭 How it works

```mermaid
flowchart LR
    A[Jellyfin] -->|Playback Stop webhook| B[Completion receiver]
    B --> C[(SQLite database)]
    D[Jellyfin metadata sync] --> C
    C --> E[Web UI]
    E -. poster fetch .-> A
```

Playback completion events are the only source of watched rows and counts. Tana never imports Jellyfin's `Played` history or `PlayCount`: tracking starts empty, and only the configured user's finished plays create rows. A webhook receiver stores accepted events, a daily sync refreshes poster and IMDb metadata from Jellyfin, and the Zola-built UI renders stats and media.

## 🚀 Quick start

1. Copy `example.env` to `.env` and set `JELLYFIN_API_KEY` (create one in Jellyfin under **Dashboard → API Keys**) and `JELLYFIN_USER_ID`. `JELLYFIN_URL` defaults to `http://jellyfin:8096` in Compose; adjust it to an address reachable from the Tana container. Non-secret settings live in the `environment:` block of `docker-compose.yaml`; only the API key is required in `.env`.
2. Create the host data directory. Compose runs the container as non-root UID/GID `10001:10001`, and the mounted `/data` directory must be writable by it:

   ```sh
   mkdir -p ./tana-data
   sudo chown -R 10001:10001 ./tana-data
   ```

3. Start and follow the logs:

   ```sh
   docker compose pull
   docker compose up -d
   docker compose logs -f tana
   ```

The example Compose file pulls the published `ghcr.io/divin/tana` image; no local Docker build is needed. It maps host port `8000` to container port `8000` on all host interfaces. Tana has no login, so only expose it to a trusted LAN or place it behind an authenticated reverse proxy, and keep the webhook endpoint private. Containers are published only on plain version tags; `:latest` always points at the newest tagged release, so `docker compose pull` after a release updates the running image.

## 🎬 Watch tracking

Tana records a movie or episode as watched only when Jellyfin sends a `PlaybackStop` event for the single user in `JELLYFIN_USER_ID` with `PlayedToCompletion=true` — or, when that flag is missing, when the stop reached `PLAYBACK_COMPLETION_THRESHOLD` (default `1.0`; valid `0.5`–`1.0`) of the runtime. Starts, progress, incomplete stops, and duplicate deliveries never increment counts; the first completion is one play and later completions count as rewatches. Configure Jellyfin's Webhook plugin as described in [Jellyfin webhook setup](docs/jellyfin-webhook.md).

## 📊 Web UI

Tana's index page shows total plays and activity over time, watched media with posters, rewatch labels, IMDb links for movies, date and kind filters, and 7/30-day statistics. Timeline data respects the configured `TZ`. The theme choice is stored locally in the browser.

## 💾 Data and privacy

The SQLite database and resized poster files live together under the mapped host directory, so back them up together (for example stop the container and `tar -czf` the directory; Tana recreates posters on demand if the `posters/` folder is removed). Webhook bodies, item IDs, and secrets are never logged. Tana only talks to its configured Jellyfin server; it reports nowhere else.

## 🧪 Development

Use Python 3.12, [`uv`](https://docs.astral.sh/uv/), Zola 0.23.6, Node.js 22, and [`just`](https://just.systems/). Copy `example.env` to `.env` (gitignored) for local runs, then from the repository root:

```sh
just sync  # Install the locked backend environment
just       # Run all checks and tests
just run   # Build the site and start a LAN-accessible server on port 8000
```

`just run` stores its database and posters in the ignored `.local-data/` directory and leaves the production `/data` path untouched. Remove `.local-data/` to discard the test copy.