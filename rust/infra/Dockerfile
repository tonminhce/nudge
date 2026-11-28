FROM rust:1-bookworm AS build
WORKDIR /build
COPY rust/Cargo.toml rust/Cargo.lock ./
COPY rust/crates ./crates
RUN cargo build --release -p nudge-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && useradd --system --uid 10001 --user-group --home-dir /nonexistent \
       --shell /usr/sbin/nologin nudge \
    && mkdir -p /var/lib/nudge-storage \
    && chown nudge:nudge /var/lib/nudge-storage \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/target/release/nudge-server /usr/local/bin/nudge-server
USER nudge
ENV NODE_PORT=3000
ENV STORAGE_LOCAL_PATH=/var/lib/nudge-storage
EXPOSE 3000
ENTRYPOINT ["nudge-server"]
