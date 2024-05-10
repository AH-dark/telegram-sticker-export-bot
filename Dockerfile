FROM rust:1.78-slim-bookworm as builder
WORKDIR /usr/src/sticker-export-bot

RUN apt update && apt install -y cmake pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

FROM debian:bookworm-slim as runner
WORKDIR /app

RUN apt update && apt install -y openssl libssl-dev ca-certificates ffmpeg && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/sticker-export-bot/target/release/sticker-export-bot /app/entry

RUN chmod +x /app/entry

RUN useradd -m appuser
USER appuser

ENTRYPOINT ["/app/entry"]
