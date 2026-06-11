# Dockerfile for MCP registry health checks (Glama et al.).
#
# Builds ONLY the `unterm-cli` package — the stdio MCP bridge — not the GUI
# (no GL / windowing deps needed). The bridge speaks the full MCP handshake
# headlessly: `initialize` and `tools/list` are served from surface tables
# compiled into the binary, and `tools/call` returns a clean "GUI not
# running" error until it can reach a live Unterm instance. That is exactly
# the behavior registry introspection checks need.
#
# This image is NOT how users run Unterm — Unterm is a desktop terminal;
# install it from https://unterm.app and the GUI auto-registers the MCP
# server with your agents.

FROM rust:1-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake pkg-config libssl-dev perl && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --release -p unterm-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/unterm-cli /usr/local/bin/unterm-cli
ENTRYPOINT ["unterm-cli", "mcp-stdio"]
