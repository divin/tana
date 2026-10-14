from __future__ import annotations

from pathlib import Path

import pytest
from pydantic import SecretStr, ValidationError

from tana.config import Settings, SettingsError, load_settings


def test_settings_load_from_env_file_and_environment_overrides(
    tmp_path: Path, monkeypatch
) -> None:
    env_file = tmp_path / ".env"
    env_file.write_text(
        "JELLYFIN_URL=http://jellyfin.example:8096/base/\n"
        "JELLYFIN_API_KEY=test-api-key\n"
        "JELLYFIN_USER_ID=test-user\n"
        "TZ=Europe/Berlin\n",
        encoding="utf-8",
    )
    monkeypatch.setenv("TZ", "UTC")

    settings = load_settings(env_file)

    assert settings.JELLYFIN_URL == "http://jellyfin.example:8096/base"
    assert settings.JELLYFIN_USER_ID == "test-user"
    assert settings.TZ == "UTC"
    assert settings.SYNC_TIME == "03:00"
    assert settings.PLAYBACK_COMPLETION_THRESHOLD == 1.0
    assert settings.LOG_LEVEL == "INFO"
    assert isinstance(settings.JELLYFIN_API_KEY, SecretStr)
    assert settings.JELLYFIN_API_KEY.get_secret_value() == "test-api-key"
    assert "test-api-key" not in repr(settings)


def test_settings_require_jellyfin_and_timezone_values(monkeypatch) -> None:
    for field in ("JELLYFIN_URL", "JELLYFIN_API_KEY", "JELLYFIN_USER_ID", "TZ"):
        monkeypatch.delenv(field, raising=False)
    with pytest.raises(ValidationError):
        Settings(
            _env_file=None,
            JELLYFIN_URL="http://jellyfin.example:8096",
            JELLYFIN_API_KEY="key",
            JELLYFIN_USER_ID="user",
        )


def test_settings_reject_invalid_url_timezone_schedule_and_log_level() -> None:
    base = {
        "_env_file": None,
        "JELLYFIN_API_KEY": "private-test-secret",
        "JELLYFIN_USER_ID": "user",
        "TZ": "Not/A_Timezone",
        "SYNC_TIME": "24:00",
        "LOG_LEVEL": "TRACE",
    }
    with pytest.raises(ValidationError):
        Settings(JELLYFIN_URL="https://user:password@example.test?token=x", **base)


def test_settings_validate_completion_threshold() -> None:
    base = {
        "_env_file": None,
        "JELLYFIN_URL": "http://jellyfin.example:8096",
        "JELLYFIN_API_KEY": "key",
        "JELLYFIN_USER_ID": "user",
        "TZ": "UTC",
    }
    with pytest.raises(ValidationError):
        Settings(PLAYBACK_COMPLETION_THRESHOLD=0.4, **base)
    with pytest.raises(ValidationError):
        Settings(PLAYBACK_COMPLETION_THRESHOLD=1.1, **base)
    with pytest.raises(ValidationError):
        Settings(PLAYBACK_COMPLETION_THRESHOLD="not-a-number", **base)
    assert (
        Settings(
            PLAYBACK_COMPLETION_THRESHOLD="0.9", **base
        ).PLAYBACK_COMPLETION_THRESHOLD
        == 0.9
    )
    assert Settings(**base).PLAYBACK_COMPLETION_THRESHOLD == 1.0


def test_load_settings_error_does_not_reveal_secret(tmp_path: Path) -> None:
    env_file = tmp_path / ".env"
    env_file.write_text(
        "JELLYFIN_URL=http://jellyfin.example\n"
        "JELLYFIN_API_KEY=do-not-print-this-secret\n"
        "JELLYFIN_USER_ID=user\n"
        "TZ=Not/A_Timezone\n",
        encoding="utf-8",
    )

    with pytest.raises(SettingsError) as error:
        load_settings(env_file)

    assert "TZ" in str(error.value)
    assert "do-not-print-this-secret" not in str(error.value)
