use crate::gemeni::generate_emojis_from_image;
use chrono::Utc;
use image::{ImageFormat, ImageReader};

use log::trace;
use std::{error::Error, io::Cursor, path::Path};
use teloxide::{
  prelude::*,
  types::{InputFile, InputSticker, MessageId, StickerFormat, UserId},
};
pub type FuncResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const MAX_STICKER_SIZE: u32 = 512;
const STICKER_FORMAT: StickerFormat = StickerFormat::Static;

pub async fn create_sticker(bot: &Bot, path: &str, user_id: UserId) -> FuncResult<InputSticker> {
  trace!("creating sticker from path: {}", path);
  if !Path::new(path).exists() {
    return Err("Sticker file not found".into());
  }

  let image = ImageReader::open(path)?.decode()?;
  trace!("got image");

  let resized = image.resize_exact(
    MAX_STICKER_SIZE,
    MAX_STICKER_SIZE,
    image::imageops::FilterType::Nearest,
  );

  let mut image_data = Cursor::new(Vec::new());
  resized.write_to(&mut image_data, ImageFormat::WebP)?;

  let uploaded_file = bot
    .upload_sticker_file(
      user_id,
      InputFile::memory(image_data.into_inner()),
      STICKER_FORMAT,
    )
    .await?;

  if uploaded_file.id.is_empty() {
    return Err("Failed to create InputSticker: uploaded file ID is empty".into());
  }

  let emoji_list = generate_emojis_from_image(path).await?;

  Ok(InputSticker {
    sticker: InputFile::file_id(uploaded_file.id),
    emoji_list: emoji_list.into_iter().take(20).collect(),
    mask_position: None,
    keywords: vec![],
  })
}

pub async fn create_sticker_set(
  bot: &Bot,
  user_id: UserId,
  sticker_set_name: &String,
  image_paths: &[String],
  progress_id: MessageId,
) -> FuncResult<()> {
  let total = image_paths.len();
  let initial_batch_size = std::cmp::min(50, total);

  let mut initial_stickers = Vec::new();
  for (i, path) in image_paths.iter().take(initial_batch_size).enumerate() {
    let processed = i + 1;
    let remaining_time = (total - processed) * 4;

    bot
      .edit_message_text(
        user_id,
        progress_id,
        format!(
          "Processing {}/{} stickers...\n⏳ ~{}s remaining",
          processed, total, remaining_time
        ),
      )
      .await?;

    let sticker = create_sticker(bot, path, user_id).await?;
    initial_stickers.push(sticker);
  }

  if initial_stickers.is_empty() {
    return Err("No valid stickers to create set".into());
  }

  bot
    .create_new_sticker_set(
      user_id,
      sticker_set_name.clone(),
      sticker_set_name.clone(),
      initial_stickers,
      STICKER_FORMAT,
    )
    .await?;

  if total > 50 {
    for (i, path) in image_paths.iter().skip(50).enumerate() {
      let processed = i + 51;
      let remaining_time = (total - processed) * 4;

      bot
        .edit_message_text(
          user_id,
          progress_id,
          format!(
            "Processing {}/{} stickers...\n⏳ ~{}s remaining",
            processed, total, remaining_time
          ),
        )
        .await?;

      let sticker = create_sticker(bot, path, user_id).await?;
      bot
        .add_sticker_to_set(user_id, sticker_set_name, sticker)
        .await?;
    }
  }

  Ok(())
}

pub async fn gen_set_name(bot: &Bot, username: &str, boardname: &str) -> FuncResult<String> {
  let botname = bot.get_me().await?.user.username.unwrap_or_default();
  let timestamp = Utc::now().timestamp_millis();
  Ok(format!(
    "{}_{}_{}_by_{}",
    username, boardname, timestamp, botname
  ))
}
