from __future__ import annotations

import re
from pathlib import Path
from urllib.parse import urlsplit
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

from pydantic import SecretStr, ValidationError, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
DOTENV_PATH = REPOSITORY_ROOT / ".env"


class SettingsError(RuntimeError):
    """Raised when settings are invalid, without including configured values."""


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=DOTENV_PATH,
        env_file_encoding="utf-8",
        extra="ignore",
    )

    JELLYFIN_URL: str
    JELLYFIN_API_KEY: SecretStr
    JELLYFIN_USER_ID: str
    TZ: str
    SYNC_TIME: str = "03:00"
    PLAYBACK_COMPLETION_THRESHOLD: float = 1.0
    JELLYFIN_ACTIVITY_LOG_ENABLED: bool = True
    LOG_LEVEL: str = "INFO"

    @field_validator("JELLYFIN_URL")
    @classmethod
    def validate_jellyfin_url(cls, value: str) -> str:
        value = value.strip()
        try:
            parts = urlsplit(value)
            port = parts.port
        except ValueError:
            raise ValueError("must be a valid absolute HTTP(S) URL") from None
        if (
            parts.scheme not in {"http", "https"}
            or not parts.hostname
            or parts.username is not None
            or parts.password is not None
            or parts.query
            or parts.fragment
            or (port is not None and not 1 <= port <= 65535)
        ):
            raise ValueError(
                "must be an absolute HTTP(S) URL without credentials, query, or fragment"
            )
        return value.rstrip("/")

    @field_validator("JELLYFIN_API_KEY")
    @classmethod
    def validate_api_key(cls, value: SecretStr) -> SecretStr:
        if not value.get_secret_value().strip():
            raise ValueError("must not be empty")
        return value

    @field_validator("JELLYFIN_USER_ID")
    @classmethod
    def validate_user_id(cls, value: str) -> str:
        value = value.strip()
        if not value:
            raise ValueError("must not be empty")
        return value

    @field_validator("TZ")
    @classmethod
    def validate_timezone(cls, value: str) -> str:
        value = value.strip()
        if not value:
            raise ValueError("must not be empty")
        try:
            ZoneInfo(value)
        except (ZoneInfoNotFoundError, ValueError):
            raise ValueError("must be a valid IANA timezone") from None
        return value

    @field_validator("SYNC_TIME")
    @classmethod
    def validate_sync_time(cls, value: str) -> str:
        if not re.fullmatch(r"(?:[01]\d|2[0-3]):[0-5]\d", value):
            raise ValueError("must use 24-hour HH:MM format")
        return value

    @field_validator("PLAYBACK_COMPLETION_THRESHOLD")
    @classmethod
    def validate_completion_threshold(cls, value: float) -> float:
        if not 0.5 <= value <= 1.0:
            raise ValueError("must be a fraction between 0.5 and 1.0")
        return value

    @field_validator("LOG_LEVEL")
    @classmethod
    def validate_log_level(cls, value: str) -> str:
        normalized = value.strip().upper()
        if normalized not in {"DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"}:
            raise ValueError("must be a supported Python logging level")
        return normalized


def load_settings(env_file: Path | None = DOTENV_PATH) -> Settings:
    """Load and validate settings while keeping secret values out of errors."""
    try:
        return Settings(_env_file=env_file)
    except ValidationError as error:
        invalid_fields = sorted(
            {
                str(issue["loc"][0]) if issue["loc"] else "configuration"
                for issue in error.errors()
            }
        )
        raise SettingsError(
            f"Invalid configuration fields: {', '.join(invalid_fields)}"
        ) from None
