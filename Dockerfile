FROM rust:1.99-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
COPY dashboard ./dashboard
COPY config ./config
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates poppler-utils && rm -rf /var/lib/apt/lists/* && useradd --uid 10001 --create-home worker && mkdir /data && chown worker:worker /data
COPY --from=build /app/target/release/drc-osint-worker /usr/local/bin/drc-osint-worker
USER worker
ENV BIND_ADDR=0.0.0.0:8080 DATABASE_URL=sqlite:///data/osint.db EVIDENCE_ARCHIVE_DIR=/data/evidence AI_SETTINGS_PATH=/data/ai-settings.json
EXPOSE 8080
VOLUME ["/data"]
CMD ["drc-osint-worker"]
