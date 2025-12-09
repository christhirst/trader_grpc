# Builder stage 
FROM docker.io/library/rust:latest AS builder

# Install protobuf compiler for tonic-build
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/trader-bot

# Copy manifests
COPY Cargo.toml ./

COPY config/ config/

# Create dummy main to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Remove dummy build artifacts
RUN rm -rf src

# Copy actual source code
COPY proto/ proto/
COPY build.rs build.rs
COPY src/ src/

# Build the actual application
# We need to touch main.rs to ensure rebuild
RUN touch src/main.rs
RUN cargo build --release

# Runtime stage
FROM docker.io/library/debian:bookworm-slim
#Copy config
COPY --from=builder /usr/src/trader-bot/config /app/config

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/trader-bot/target/release/trader-bot /app/trader-bot

# Set entrypoint
CMD ["./trader-bot"]
