# pinsToStickers ([bot link](https://t.me/pinsToStickers_bot))

Just one simple command:
`/createset <pinterest board url>`

# Self-hosting instructions:

## Docker image

```
docker pull #TODO ADD A LINK TO THE IMAGE
```

## Running locally

1. [Download Rust](http://rustup.rs/).
2. Create a new bot using [@Botfather](https://t.me/botfather) to get a token in the format `123456789:blablabla`.
3. Clone the repo

```bash
git clone https://github.com/xhos/pinsToStickers.git
```

4. Initialise the `TELOXIDE_TOKEN` environmental variable to your token:

```bash
# Unix-like
$ export TELOXIDE_TOKEN=<Your token here>

# Windows command line
$ set TELOXIDE_TOKEN=<Your token here>

# Windows PowerShell
$ $env:TELOXIDE_TOKEN=<Your token here>
```

5. Run it!

```bash
cargo run
```
