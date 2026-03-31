# Stage 1: Build stage
FROM rust:1.93.1-alpine AS builder

# Install build dependencies for compilation
RUN apk add --no-cache \
    musl-dev \
    sqlite-dev \
    sqlite-libs \
    curl \
    pkgconfig

# Create app directory
WORKDIR /app

# Copy Cargo manifest files (paths relative to root context)
COPY backend/Cargo.toml backend/Cargo.lock ./

# Copy source code
COPY backend/src ./src

# Copy migrations
COPY backend/migrations ./migrations

# Build release binary with locked dependencies
RUN cargo build --release --locked

# Verify binary was created successfully
RUN ls -lh /app/target/release/tana


# Stage 2: Runtime stage - minimal alpine image
FROM alpine:3.20

# Install only essential runtime dependencies
RUN apk add --no-cache \
    libssl3 \
    sqlite-libs \
    curl \
    ca-certificates

# Create non-root user for security best practices
RUN addgroup -S tana && adduser -S tana -G tana

# Create app directory
WORKDIR /app

# Copy only the compiled binary from builder stage
COPY --from=builder /app/target/release/tana /app/tana

# Create data directory for persistent storage
RUN mkdir -p /home/tana/.local/share/tana && \
    chown -R tana:tana /home/tana && \
    chown -R tana:tana /app

# Switch to non-root user
USER tana

# Expose API port
EXPOSE 8080

# Health check to verify service availability
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl --connect-timeout 2 --max-time 3 -f http://localhost:8080/api/stats || exit 1

# Run the server with network accessible to all interfaces
CMD ["./tana", "serve", "--host", "0.0.0.0", "--port", "8080"]
