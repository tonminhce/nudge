FROM rust:1-bookworm AS build
WORKDIR /build
COPY rust/Cargo.toml rust/Cargo.lock ./
COPY rust/crates ./crates
RUN cargo build --release -p nudge-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/target/release/nudge-server /usr/local/bin/nudge-server
ENV NODE_PORT=3000
EXPOSE 3000
ENTRYPOINT ["nudge-server"]
