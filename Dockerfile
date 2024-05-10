FROM rust:1.78-bookworm as builder
WORKDIR /usr/src/sticker-export-bot

RUN rustup default nightly

COPY . .

RUN apt update -y && apt install -y cmake

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release

FROM debian:bookworm-slim as runner
WORKDIR /app

RUN apt update -y
RUN apt install -y openssl libssl-dev ca-certificates ffmpeg
RUN rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/sticker-export-bot/target/release/sticker-export-bot /app/entry

USER root
RUN chmod +x /app/entry

ENTRYPOINT ["/app/entry"]
