set dotenv-load := false

# Default: verify the backend and frontend, then run their tests.
default: check test frontend-test

# Format Python backend code.
fmt:
    cd backend && uv run ruff format .

# Lint and automatically fix safe issues.
lint:
    cd backend && uv run ruff check --fix .

# Sort Python imports with Ruff.
isort:
    cd backend && uv run ruff check --select I --fix .

# Run non-mutating checks for the backend and Zola site.
check: frontend-check
    cd backend && uv run ruff format --check .
    cd backend && uv run ruff check .

# Run backend tests, building the Zola site required by the app test.
test: frontend-build
    cd backend && uv run pytest tests -q

# Validate Zola templates and links.
frontend-check:
    zola --root frontend check --skip-external-links

# Build the static site.
frontend-build:
    zola --root frontend build

# Run dependency-free frontend tests.
frontend-test: frontend-build
    cd frontend && node --test

# Run a local instance bound only to loopback, storing test data in .local-data/.
run: frontend-build
    cd backend && uv run uvicorn tana.dev:app --host 0.0.0.0 --port 8000 --workers 1

# Install/update the locked Python environment.
sync:
    cd backend && uv sync
