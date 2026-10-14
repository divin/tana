# Tana

A Jellyfin watch-history tracker. The backend is in `backend/` and the Zola site is in `frontend/`; see [`spec/README.md`](spec/README.md) and [`spec/roadmap.md`](spec/roadmap.md) for the implementation contract and release plan.

## Development

Install Python 3.12, [`uv`](https://docs.astral.sh/uv/), Zola 0.23.6, Node.js 22, and [`just`](https://just.systems/). Zola builds the static site directly; no npm install is needed.

Copy `example.env` to `.env` if you do not already have one, then set `JELLYFIN_URL`, `JELLYFIN_API_KEY`, `JELLYFIN_USER_ID`, and `TZ`. The `.env` file is local configuration and is ignored by Git. For development with `just run`, keep the full set in `.env`; for Docker Compose deployments, non-secret settings are read from the `environment:` block in `docker-compose.yaml` and only `JELLYFIN_API_KEY` is required in `.env`.

From the repository root:

```sh
just sync  # Install the locked backend environment
just       # Run checks and tests
just run   # Build the site and start a LAN-accessible local server on port 8000
```

`just run` refreshes Jellyfin metadata and stores its local test database/posters in the ignored `.local-data/` directory, leaving the production container path `/data` untouched. The history starts empty; only successful completion webhooks create watched items. Configure Jellyfin's Webhook plugin as described in [`docs/jellyfin-webhook.md`](docs/jellyfin-webhook.md). Stop it with Ctrl+C. It binds to `0.0.0.0:8000`, so you can open `http://<computer-lan-ip>:8000` from another device on your trusted LAN. The app has no login; only run it on a trusted network and allow the port through your firewall only as needed. Remove `.local-data/` to discard the test copy.

## Docker Compose deployment

Tana has no user login. Only expose it to a trusted LAN or place it behind an authenticated reverse proxy; do not expose it directly to the public internet.

1. Install Docker Engine/Desktop with the Compose plugin. Non-secret configuration is set directly in `docker-compose.yaml` under `environment:` (Jellyfin URL, user ID, timezone, sync time, completion threshold, log level). The only value that must live in `.env` is the API key: copy `example.env` to `.env` if you have not already done so, then set
   - `JELLYFIN_API_KEY`: create one in Jellyfin under **Dashboard → API Keys**. Give it a name such as `Tana` and copy the generated key into `.env`; never commit or share it.

   If you fork or adapt the example, also adjust the values in the `environment:` block — `JELLYFIN_URL` is the base URL for your Jellyfin server that is reachable from inside the Tana container (in Compose, `localhost` points to Tana itself, so use a LAN address, a Compose service name, or a host-gateway alias), `JELLYFIN_USER_ID` is the Jellyfin user whose watched history to track, `TZ` is an IANA timezone such as `Europe/Berlin`, and `PLAYBACK_COMPLETION_THRESHOLD` (optional) is the fraction of runtime between `0.5` and `1.0` that a stop must reach to count as watched when Jellyfin's `PlayedToCompletion` flag is not set (default `1.0`).
2. `docker-compose.yaml` already points at `ghcr.io/divin/tana`. Images are published only when a plain version tag such as `1.0.0` is pushed; `:latest` always points at the newest tagged release. To pin a specific release, edit `image:` to a plain version tag such as `:1.0.0`. To choose another data location, change the **left side** of `./tana-data:/data` to the directory on your server; the container path on the right must stay `/data`.
3. Create the host data directory and make it writable by the container's non-root UID/GID `10001:10001` (on Linux):

   ```sh
   mkdir -p ./tana-data
   sudo chown -R 10001:10001 ./tana-data
   ```

4. Start Tana:

   ```sh
   docker compose pull
   docker compose up -d
   docker compose logs -f tana
   ```

   The Compose file is the end-user deployment example and always pulls the published image from GHCR; it does not build from source. Images are published only on version-tag pushes (`:latest` tracks the newest tag), so pull after `git tag`-driven releases. For local development from a checkout, use `just run` instead (see Development). Choose the desired tag by editing `image:` in `docker-compose.yaml` (for example `:0.9.0` for a specific release instead of `:latest`).

The container listens on port `8000`. The Compose example publishes `8000:8000` on host interfaces, so configure your firewall to allow only trusted devices/networks. GitHub Container Registry packages are private by default when first published: to let a server pull without credentials, open the package settings on GitHub and change its visibility to Public. Otherwise, authenticate Docker on the server with a GitHub token that has `read:packages` access before pulling. The database is stored inside the container at `/data/tana.sqlite3`; resized poster files are in `/data/posters`. The volume mapping keeps both across container replacement. Without it, both are ephemeral.

### Backup and restore

Back up the complete mapped host data directory so the database and poster cache remain together. For a simple consistent offline backup:

```sh
docker compose stop tana
tar -czf tana-data-backup.tar.gz -C ./tana-data .
docker compose start tana
```

To restore, stop the container, replace the contents of the mapped directory from the backup, restore writable ownership for UID/GID `10001:10001`, then start the service. Removing `/data/posters` alone is safe; Tana will fetch posters again on demand. Protect backups as personal data.

### Jellyfin webhook

Configure the Generic Destination after the webhook receiver is present in the deployed image. Use the exact fields and delivery behavior in [Jellyfin webhook setup](docs/jellyfin-webhook.md). No webhook auth token is required; restrict access to Tana to your trusted LAN.

### Upgrade

Change the image tag in Compose to the desired plain version (for example `1.0.0`), then run `docker compose pull` and `docker compose up -d`. Keep the `/data` mapping unchanged. The service currently uses schema version 3; future database schema changes will include explicit migrations and upgrade instructions.
