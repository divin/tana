"""Local-development ASGI app using a disposable repository-local data directory."""

from pathlib import Path

from tana.app import create_app

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
app = create_app(data_dir=REPOSITORY_ROOT / ".local-data")
