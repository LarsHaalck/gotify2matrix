###################
# chef
###################
FROM lukemathwalker/cargo-chef:latest-rust-1.74 AS chef
WORKDIR /app

###################
# planner
###################
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

###################
# builder
###################
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

###################
# runtime
###################
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y \
    libsqlite3-0 \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/gotify2matrix /usr/local/bin
ENV RUST_LOG="warn,gotify2matrix=debug"
CMD ["/usr/local/bin/gotify2matrix"]
