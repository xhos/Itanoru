// use futures::future::join_all;
// use teloxide::payloads::CreateNewStickerSet;
// use teloxide::payloads::UploadStickerFile;
use image::io::Reader as ImageReader;
use rand::seq::SliceRandom;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command as SystemCommand;
use std::{error::Error, io::Cursor};
use teloxide::{prelude::*, types::InputFile, types::InputSticker, utils::command::BotCommands};

const MAX_STICKER_SIZE: u32 = 512;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting sticker bot...");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Displays this text")]
    Help,
    #[command(description = "Creates a sticker set from a URL to a Pinterest board")]
    CreateSet(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            send_help_message(bot, msg).await?
        }
        Command::CreateSet(url) => {
            handle_pinterest_sticker_set(&bot, &url).await?
        }
    };

    Ok(())
}

async fn send_help_message(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await
}

async fn handle_pinterest_sticker_set(bot: &Bot, url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    download_board(url)?;
    let image_paths = get_image_paths(url)?;
    create_sticker_set(bot, &url.to_string(), &image_paths).await?;
    Ok(())
}

async fn create_sticker_set(bot: &Bot, url: &String, image_paths: &[String]) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (username, boardname) = get_pinterest_info(url)?;
    let bot_id = bot.get_me().await?.user.id;
    let sticker_set_name = format!("{}_{}_by_{}", username, boardname, bot_id);

    // Create a new sticker set
    let sticker = InputSticker::Png(InputFile::file_id("")); // replace with a valid sticker
    let emojis = get_random_emoji();

    bot.create_new_sticker_set(bot_id, sticker_set_name.clone(), username.clone(), sticker, emojis.clone()).await?;

    for path in image_paths {
        let image = ImageReader::open(path)?.decode()?;
        let resized = image.thumbnail(MAX_STICKER_SIZE, MAX_STICKER_SIZE);
        let mut image_data = Cursor::new(Vec::new());
        resized.write_to(&mut image_data, image::ImageOutputFormat::Png)?;
        let input_file = InputFile::memory(image_data.into_inner());

        let uploaded_file = bot.upload_sticker_file(bot_id, input_file).await?;
        let input_sticker = InputSticker::Png(InputFile::file_id(uploaded_file.id));

        bot.add_sticker_to_set(bot_id, sticker_set_name.clone(), input_sticker, emojis.clone()).await?;
    }

    Ok(())
}

// Returns the username and boardname from a Pinterest URL
fn get_pinterest_info(url: &str) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
    let output = SystemCommand::new("./src/gallery-dl.exe")
        .arg("-j")
        .arg(url)
        .output()?;

    if !output.status.success() {
        return Err("gallery-dl.exe systemcommand failed".into());
    }

    let json: Value = serde_json::from_slice(&output.stdout)?;
    let url = json[0][1]["board"]["url"]
        .as_str()
        .ok_or("Invalid URL format")?;

    let parts: Vec<&str> = url.split('/').collect();
    let username = parts[1].to_string();
    let board_name = parts[2].to_string();

    Ok((username, board_name))
}

// Downloads the Pinterest board using gallery-dl into gallery-dl/pinterest/username/boardname
fn download_board(url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let output = SystemCommand::new("./src/gallery-dl.exe")
        .arg(url)
        .output()?;

    if !output.status.success() {
        return Err("gallery-dl.exe systemcommand failed".into());
    }

    Ok(())
}

// Returns a vector of image paths in the gallery-dl/pinterest/username/boardname directory
fn get_image_paths(url: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let (username, board_name) = get_pinterest_info(url)?;

    let dir_path = format!("gallery-dl/pinterest/{}/{}", username, board_name);
    let dir = Path::new(&dir_path);

    if !dir.is_dir() {
        return Err("Invalid directory path".into());
    }

    let mut paths = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            paths.push(entry.path().to_string_lossy().into_owned());
        }
    }

    Ok(paths)
}

fn get_random_emoji() -> String {
    let emojis = ["😀", "😃", "😄", "😁", "😆", "😅", "😂", "🤣", "🙂", "🙃"];
    let mut rng = rand::thread_rng();
    emojis.choose(&mut rng).unwrap().to_string()
}

// async fn create_and_add_sticker(
//     bot: Bot,
//     user_id: UserId,
//     chat_id: ChatId,
//     sticker_set_name: &str,
//     image_path: &str,
// ) -> Result<(), Box<dyn Error + Send + Sync>> {
//     let image = ImageReader::open(image_path)?.decode()?;
//     let resized = image.thumbnail(MAX_STICKER_SIZE, MAX_STICKER_SIZE);
//     let mut image_data = Cursor::new(Vec::new());

//     resized.write_to(&mut image_data, image::ImageOutputFormat::Png)?;
//     let input_file = InputFile::memory(image_data.into_inner());

//     let uploaded_file = bot.upload_sticker_file(user_id, input_file).await?;

//     let input_sticker = InputSticker::Png(InputFile::file_id(uploaded_file.id));

//     let emoji = get_random_emoji();

//     let result = bot.add_sticker_to_set(user_id, sticker_set_name, input_sticker, &emoji)
//     .await
//     .map_err(|e| -> Box<dyn Error + Send + Sync> { e.into() })?;

//     Ok(())
// }

// fn encode_image_to_png(image: image::DynamicImage) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
//     let mut buffer = Cursor::new(Vec::new());
//     image.write_to(&mut buffer, image::ImageOutputFormat::Png)?;
//     Ok(buffer.into_inner())
// }

