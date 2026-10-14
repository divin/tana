from __future__ import annotations

from fastapi import APIRouter, HTTPException, Request, Response
from starlette.requests import ClientDisconnect

from tana.webhooks import MAX_WEBHOOK_BYTES, WebhookPayloadError, WebhookService

router = APIRouter()


@router.post("/api/webhooks/jellyfin", status_code=204)
async def receive_jellyfin_webhook(request: Request) -> Response:
    content_type = (
        request.headers.get("content-type", "").split(";", 1)[0].strip().lower()
    )
    if content_type != "application/json":
        raise HTTPException(
            status_code=415, detail="Content-Type must be application/json"
        )

    content_length = request.headers.get("content-length")
    if content_length is not None:
        try:
            if int(content_length) > MAX_WEBHOOK_BYTES:
                raise HTTPException(status_code=413, detail="Webhook payload too large")
        except ValueError:
            raise HTTPException(
                status_code=400, detail="Invalid webhook request"
            ) from None

    body = bytearray()
    try:
        async for chunk in request.stream():
            if len(body) + len(chunk) > MAX_WEBHOOK_BYTES:
                raise HTTPException(status_code=413, detail="Webhook payload too large")
            body.extend(chunk)
    except HTTPException:
        raise
    except ClientDisconnect:
        raise HTTPException(status_code=400, detail="Invalid webhook request") from None

    service: WebhookService = request.app.state.webhook_service
    try:
        await service.handle_json(bytes(body))
    except WebhookPayloadError:
        raise HTTPException(status_code=400, detail="Invalid webhook payload") from None
    return Response(status_code=204)
