# Telegram Sticker Export Bot

This is a simple bot that allows you to export stickers from Telegram to PNG or GIF files.

## Usage

1. Start the bot by running `cargo run`.
2. Use bot with commands:
    - `/start` - Start the bot.
    - `/single` - Export single sticker.
    - `/pack` - Export all stickers from a pack.
    - `/cancel` - Cancel the current operation.

## Configuration

The bot requires some environment variables to be set:

- `TELOXIDE_TOKEN` - Telegram bot token.
- `TELEGRAM_API_URL` - Telegram API URL. Default is `https://api.telegram.org`.
- `OTEL_EXPORTER_ENDPOINT` - The endpoint of the OpenTelemetry exporter (default: `http://localhost:4317`)
- `OTEL_EXPORTER` - The type of the OpenTelemetry exporter (default: `otlp_grpc`, available: `otlp_grpc`, `otlp_http`)
- `OTEL_SAMPLE_RATE` - The sample rate of the OpenTelemetry exporter (default: `1.0`)
- `RUST_LOG` - The log level of the application (available: `trace`, `debug`, `info`, `warn`, `error`)

## License

This project is licensed under the Affero General Public License v3.0 - see the [LICENSE](LICENSE) file for details.
