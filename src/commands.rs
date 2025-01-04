use crate::stickers::{self, create_sticker_set, FuncResult};
use log::{error, trace};
use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(
  rename_rule = "lowercase",
  description = "These commands are available:"
)]
pub enum Command {
  #[command(description = "Display help message")]
  Help,
  #[command(description = "Create sticker set from Pinterest board URL")]
  CreateSet(String),
}

pub async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
  match cmd {
    Command::Help => {
      handle_help_command(bot, msg).await?;
    }
    Command::CreateSet(url) => match handle_create_set(&bot, msg.from.unwrap().id, &url).await {
      Ok(_) => (),
      Err(e) => {
        error!("Failed to handle Pinterest sticker set: {}", e);
        bot
          .send_message(msg.chat.id, "Failed to create sticker set")
          .await?;
      }
    },
  }
  Ok(())
}

async fn handle_help_command(bot: Bot, msg: Message) -> ResponseResult<()> {
  trace!("handling help command");
  bot
    .send_message(msg.chat.id, Command::descriptions().to_string())
    .await?;
  Ok(())
}

pub async fn handle_create_set(bot: &Bot, user_id: UserId, url: &str) -> ResponseResult<()> {
  match create_set_internal(bot, user_id, url).await {
    Ok(_) => Ok(()),
    Err(e) => {
      error!("Failed to create sticker set: {}", e);
      bot
        .send_message(user_id, "Failed to create sticker set")
        .await?;
      Ok(())
    }
  }
}

async fn create_set_internal(bot: &Bot, user_id: UserId, url: &str) -> FuncResult<()> {
  let (username, boardname, _) = crate::pinterest::get_pinterest_info(url)?;
  let initial_count = crate::pinterest::get_image_count(url)?;

  if initial_count > 120 {
    bot
      .send_message(user_id, "Too many images! Please limit to 120 or fewer.")
      .await?;
    return Ok(());
  }

  let progress = bot.send_message(user_id, "Downloading images...").await?;
  crate::pinterest::download_board(url)?;

  let image_paths = crate::pinterest::get_image_paths(&username, &boardname)?;
  let sticker_set_name = stickers::gen_set_name(bot, &username, &boardname).await?;

  match create_sticker_set(bot, user_id, &sticker_set_name, &image_paths, progress.id).await {
    Ok(_) => {
      let sticker_set_url = format!("https://t.me/addstickers/{}", sticker_set_name);
      bot
        .edit_message_text(
          user_id,
          progress.id,
          format!("Sticker set created: {sticker_set_url}"),
        )
        .await?;
      Ok(())
    }
    Err(e) => {
      if let Err(cleanup_err) = crate::pinterest::cleanup_data(&username, &boardname) {
        error!("Failed to cleanup data: {}", cleanup_err);
      }
      Err(e)
    }
  }
}
