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

# Create non-root user for security best practices with explicit uid/gid
RUN addgroup -g 1000 tana && \
    adduser -u 1000 -G tana -s /sbin/nologin -D tana

# Create app directory
WORKDIR /app

# Copy only the compiled binary from builder stage
COPY --from=builder /app/target/release/tana /app/tana

# Create data directory for persistent storage with proper permissions
RUN mkdir -p /home/tana/.local/share/tana && \
    chmod 755 /home/tana && \
    chmod 755 /home/tana/.local && \
    chmod 755 /home/tana/.local/share && \
    chmod 755 /home/tana/.local/share/tana && \
    chown -R 1000:1000 /home/tana && \
    chown -R 1000:1000 /app && \
    chmod 755 /app

# Switch to non-root user
USER tana

# Expose API port
EXPOSE 8080

# Run the server with network accessible to all interfaces
CMD ["./tana", "serve", "--host", "0.0.0.0", "--port", "8080"]
