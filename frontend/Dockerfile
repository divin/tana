# Stage 1: Build SvelteKit application
FROM node:25-alpine AS builder

WORKDIR /app

# Copy package files
COPY frontend/package*.json ./

# Install all dependencies
RUN npm ci

# Copy frontend source
COPY frontend/src ./src
COPY frontend/static ./static
COPY frontend/svelte.config.js ./
COPY frontend/vite.config.js ./
COPY frontend/vite.config.ts ./
COPY frontend/tsconfig.json ./

# Generate SvelteKit files
RUN npx svelte-kit sync

# Build the application
RUN npm run build

# Stage 2: Runtime
FROM node:25-alpine

WORKDIR /app

# Copy package files
COPY frontend/package*.json ./

# Install production dependencies and express for custom server
RUN npm ci --omit=dev && npm install express http-proxy

# Copy built application from builder
COPY --from=builder /app/build ./build

# Copy the pre-made server.js file
COPY frontend/server.js /app/server.js

EXPOSE 3000

ENV NODE_ENV=production

CMD ["node", "/app/server.js"]
