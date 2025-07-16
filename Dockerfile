# Multi-stage Docker build for Code Mesh

# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy dependency files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build dependencies first (cached layer)
RUN cargo build --release --bin code-mesh

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false code-mesh

# Copy binary from builder stage
COPY --from=builder /app/target/release/code-mesh /usr/local/bin/code-mesh

# Set permissions
RUN chmod +x /usr/local/bin/code-mesh

# Switch to app user
USER code-mesh

# Set working directory
WORKDIR /home/code-mesh

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD code-mesh status || exit 1

# Default command
ENTRYPOINT ["code-mesh"]
CMD ["--help"]

# Metadata
LABEL org.opencontainers.image.title="Code Mesh"
LABEL org.opencontainers.image.description="AI-powered coding assistant built in Rust"
LABEL org.opencontainers.image.vendor="ruvnet"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.source="https://github.com/ruvnet/code-mesh"