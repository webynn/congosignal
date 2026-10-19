FROM rust:1.99-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
COPY dashboard ./dashboard
COPY config ./config
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates poppler-utils curl python3 gosu && rm -rf /var/lib/apt/lists/* && useradd --uid 10001 --create-home worker && mkdir /data && chown worker:worker /data
COPY --from=build /app/target/release/drc-osint-worker /usr/local/bin/drc-osint-worker
COPY scripts/backup.py /usr/local/bin/congo-backup.py
COPY --chmod=755 scripts/container-start.sh /usr/local/bin/container-start
WORKDIR /data
USER worker
ENV DATABASE_URL=sqlite:///data/osint.db EVIDENCE_ARCHIVE_DIR=/data/evidence AI_SETTINGS_PATH=/data/ai-settings.json AGREEMENTS_ARCHIVE_DIR=/data/agreements
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/container-start"]
CMD ["drc-osint-worker"]
