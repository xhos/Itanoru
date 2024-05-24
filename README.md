# pinsToStickers ([bot link](https://t.me/pinsToStickers_bot))
Just one simple command:
```/createset <pinterest board url>```

# Self-hosting instructions:
## Docker image
```
docker pull #TODO ADD A LINK TO THE IMAGE
```
## Running locally
- Install Rust (command bellow if for unix systems only, if you are running on windows it is recommended to use WSL, if you can't - refer to the official [rust installation intrustivons](https://www.rust-lang.org/learn/get-started)
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
- Clone the repo
```
git clone https://github.com/xhos/pinsToStickers.git
```
- Run it!
```
cargo run
```
