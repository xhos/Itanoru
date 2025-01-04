mod commands;
mod gemeni;
mod pinterest;
mod stickers;

use commands::{answer, Command};
use log::info;
use teloxide::{repls::CommandReplExt, Bot};

#[tokio::main]
async fn main() {
  pretty_env_logger::init();
  info!("Starting sticker bot...");

  let bot = Bot::from_env();
  Command::repl(bot, answer).await;
}
