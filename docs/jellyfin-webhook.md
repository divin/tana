# Jellyfin webhook setup 🕸️

**Tana counts completed plays from Jellyfin's Webhook plugin — not from `UserData.PlayCount`.** Jellyfin bumps that counter on playback *start* and may update `LastPlayedDate` while watching, so it cannot tell a finished movie from a dropped one. Tana only accepts Playback Stop notifications for the single user in `JELLYFIN_USER_ID`: events with `PlayedToCompletion=true`, plus unflagged stops that reached `PLAYBACK_COMPLETION_THRESHOLD` (default `1.0`, meaning the full runtime) of `RunTimeTicks`. Jellyfin normally flags completions itself, so the threshold only adds leniency — it never rejects an event Jellyfin already flagged as complete.

## Generic Destination settings

In Jellyfin's Dashboard, open the Webhook plugin configuration and add a **Generic Destination**:

| Setting | Value |
|---|---|
| Webhook Name | `Tana completed plays` |
| Webhook Url | `http://<TANA_LAN_HOST>:8000/api/webhooks/jellyfin` |
| Status | Enable after Tana is running with this endpoint implemented |
| Notification Type | `Playback Stop` only |
| User Filter | Select the Jellyfin account matching `JELLYFIN_USER_ID` if the plugin's selector is available; Tana also validates `UserId` server-side |
| Item Type | `Movies` and `Episodes` |
| Send All Properties | Enabled; required so `PlayedToCompletion`, `UserId`, `ItemId`, `ItemType`, and `UtcTimestamp` are sent |
| Trim leading and trailing whitespace | Enabled |
| Do not send when message body is empty | Enabled |
| Headers | Add `Content-Type` with value `application/json`. Do **not** add an `Authorization` header or token |

Replace `<TANA_LAN_HOST>` with an address Jellyfin can reach. On a host where Tana's Compose service maps port `8000:8000`, use the host's LAN address, for example `http://192.168.8.178:8000/api/webhooks/jellyfin` (use the current address if it changed). When Tana and Jellyfin run as separate Compose services on a shared network, use Tana's service name and container port instead, for example `http://tana:8000/api/webhooks/jellyfin`.

## 🔐 Authentication and network access

The webhook endpoint intentionally has no authentication, matching Tana's no-login design. Do not expose port `8000` or this endpoint to the public internet; restrict access to a trusted LAN with the host firewall. `JELLYFIN_USER_ID` is a correctness filter, not an authentication mechanism — any device that can reach the route could forge a completion event, so keep the route private.

## 🌱 Fresh tracking behavior

Tana does not backfill historical plays. On its first run the database is empty (a version-1 or version-2 database from older Tana builds is reset), and the initial Jellyfin sync creates no watched rows from past `Played` items. A row is created only when Tana receives a valid completed Movie/Episode stop event. New rows are enriched with their Jellyfin poster and IMDb metadata immediately (best-effort); if Jellyfin is unreachable the row is still recorded and the next scheduled sync fills the metadata in. The first confirmed completion is one play; later confirmed completions of the same item count as rewatches. Starts, progress, pauses, resumes, incomplete stops, and repeated deliveries of the same event do not increment counts.

The `/api/webhooks/jellyfin` endpoint returns `204` for accepted, duplicate, and valid-but-ignored events. It is implemented in the current image, so webhook requests sent while an older Tana was running (which may have returned `404`) are not replayed by Jellyfin.