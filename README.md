# Itanoru

A bot that creates a Telegram sticker set from a Pinterest board. Also assigns the emojis for stickers using AI, allowing to find the right sticker faster.

There's a version that I host. [link](https://t.me/ItanoruBot)
Or you can host the bot yourself.

## Usage

```bash
/createset <pinterest board/section link>
```

## Self hosting

### Prerequisites

- [Telegram Bot Token](https://t.me/botfather)
- [Google Gemini API Key](https://aistudio.google.com/app/apikey)

### Docker

```bash
docker run -d \
  --name itanoru \
  -e TELOXIDE_TOKEN="YOUR_BOT_TOKEN_HERE" \
  -e GEMINI_TOKEN="GEMENI_TOKEN_HERE" \
  -v itanoru-data:/app/data \
  ghcr.io/xhos/itanoru
```

### Compiling

- [gallery-dl](https://github.com/mikf/gallery-dl) available in the environment

1. Clone the repository

```bash
git clone https://github.com/yourusername/Itanoru.git
cd Itanoru
```

2. Configure environment variables

```
TELOXIDE_TOKEN="your_telegram_bot_token"
GEMINI_TOKEN="your_gemini_api_key"
```

3. Build and run

```bash
cargo run --release
```

## Development

```bash
$env:RUST_LOG="trace"
cargo run
```
