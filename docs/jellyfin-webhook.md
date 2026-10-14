# Jellyfin completed-play webhook setup

Tana counts completed plays from Jellyfin's Webhook plugin, not `UserData.PlayCount`. Jellyfin increments that counter on playback start, and may update `LastPlayedDate` during progress. The events Tana accepts are Playback Stop notifications for the single user in `JELLYFIN_USER_ID`: those with `PlayedToCompletion=true`, plus unflagged stops that reached `PLAYBACK_COMPLETION_THRESHOLD` (default `1.0`, i.e. full runtime) of `RunTimeTicks`. Jellyfin normally flags completions itself, so the threshold only adds leniency, never rejects flagged events.

## Generic Destination settings

In Jellyfin's Dashboard, open the Webhook plugin configuration and add a **Generic Destination**:

| Setting | Value |
|---|---|
| Webhook Name | `Tana completed plays` |
| Webhook Url | `http://<TANA_LAN_HOST>:8000/api/webhooks/jellyfin` |
| Status | Enable after restarting Tana with a build/image that implements this endpoint |
| Notification Type | `Playback Stop` only |
| User Filter | Select the Jellyfin account matching `JELLYFIN_USER_ID` (currently `admin`) if the plugin's selector is available. Tana also validates `UserId` server-side. |
| Item Type | `Movies` and `Episodes` |
| Send All Properties | Enabled; required so `PlayedToCompletion`, `UserId`, `ItemId`, `ItemType`, and `UtcTimestamp` are sent. |
| Trim leading and trailing whitespace | Enabled |
| Do not send when message body is empty | Enabled |
| Headers | Add `Content-Type` with value `application/json`. Do **not** add an `Authorization` header or token. |

Replace `<TANA_LAN_HOST>` with an address Jellyfin can reach. For example, when Tana is published on the MacBook shown in the local-development instructions, use `http://192.168.8.178:8000/api/webhooks/jellyfin`; use the correct current LAN address if it changes. With separate Compose containers on a shared network, use Tana's service name and container port instead.

## Authentication and network access

The webhook endpoint intentionally has no authentication, matching Tana's no-auth design. Do not expose port 8000 or this endpoint to the public internet. Restrict access to a trusted LAN with the host firewall. The configured Jellyfin user ID is a correctness filter, not an authentication mechanism; another LAN device able to reach the route could forge a completion event.

## Fresh tracking behavior

Tana does not backfill historical plays. On schema cutover, version-1/version-2 watched history and poster cache are reset. The initial Jellyfin sync does not create watched rows from old `Played` items. A row is created only when Tana receives a valid completed Movie/Episode stop event (`PlayedToCompletion=true`, or a stop meeting `PLAYBACK_COMPLETION_THRESHOLD`). New rows are enriched with their Jellyfin poster/IMDb metadata immediately (best-effort); if Jellyfin is unreachable the row is still recorded and the next scheduled sync fills the metadata in. The first confirmed completion is one play; further confirmed completions are counted as rewatches. Starts, progress, pauses, resumes, incomplete stops, and repeated deliveries of the same event do not increment counts.

The `/api/webhooks/jellyfin` endpoint is implemented in the current source. If Tana was already running before this update, restart it (or rebuild/restart its container) before relying on delivery; webhook requests sent while the old app was running may have received 404 and are not replayed. The endpoint returns `204` for accepted, duplicate, and valid-but-ignored events.
