# syntax=docker/dockerfile:1.7

FROM python:3.12-slim AS build
ARG TARGETARCH

ENV UV_NO_PROGRESS=1 \
    UV_COMPILE_BYTECODE=1

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# Pin the exact Zola release binary and verify its published SHA-256 digest.
RUN set -eux; \
    case "${TARGETARCH:-$(uname -m)}" in \
      x86_64|amd64) zola_arch="x86_64"; zola_sha="8f5132b3522412d04e395e0b25f6d68613ad272a873e54a2b3ebf664873024a4" ;; \
      aarch64|arm64) zola_arch="aarch64"; zola_sha="266448fffbf7c7004ca399d0e76dd699541771096d8a42aede98cebe2a029d02" ;; \
      *) echo "Unsupported Docker architecture: ${TARGETARCH:-$(uname -m)}" >&2; exit 1 ;; \
    esac; \
    archive="zola-v0.23.6-${zola_arch}-unknown-linux-gnu.tar.gz"; \
    curl --fail --silent --show-error --location \
      "https://github.com/getzola/zola/releases/download/v0.23.6/${archive}" \
      --output "/tmp/${archive}"; \
    echo "${zola_sha}  /tmp/${archive}" | sha256sum --check --status; \
    tar -xzf "/tmp/${archive}" -C /usr/local/bin zola; \
    rm "/tmp/${archive}"; \
    zola --version

RUN pip install --no-cache-dir uv==0.12.23

WORKDIR /app/backend
COPY backend/ ./
RUN uv sync --frozen --no-dev --no-editable

WORKDIR /app/frontend
COPY frontend/ ./
RUN zola check --skip-external-links && zola build

FROM python:3.12-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PATH="/app/backend/.venv/bin:${PATH}"

RUN groupadd --gid 10001 tana \
    && useradd --uid 10001 --gid tana --no-create-home --shell /usr/sbin/nologin tana \
    && mkdir -p /data \
    && chown 10001:10001 /data

WORKDIR /app/backend
COPY --from=build --chown=10001:10001 /app/backend/.venv /app/backend/.venv
COPY --from=build --chown=10001:10001 /app/frontend/public /app/frontend/public

VOLUME ["/data"]
EXPOSE 8000
USER 10001:10001

HEALTHCHECK --interval=30s --timeout=3s --start-period=30s --retries=3 \
  CMD ["python", "-c", "import urllib.request; urllib.request.urlopen('http://127.0.0.1:8000/healthz', timeout=2)"]

ENTRYPOINT ["tana"]
