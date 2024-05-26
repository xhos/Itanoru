use std::fs;
use std::path::Path;
use serde_json::Value;
use rand::seq::SliceRandom;
use std::{error::Error, io::Cursor};
use image::io::Reader as ImageReader;
use std::process::Command as SystemCommand;
use teloxide::{prelude::*, types::{InputFile, UserId, InputSticker, Message, ChatId, StickerSet},utils::command::BotCommands};

const MAX_STICKER_SIZE: u32 = 512;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting sticker bot...");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are available:")]
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
            match handle_pinterest_sticker_set(&bot, msg.from().unwrap().id, &url).await {
                Ok(_) => (),
                Err(e) => {
                    log::error!("Failed to handle Pinterest sticker set: {}", e);
                    bot.send_message(msg.chat.id, format!("Failed to create sticker set: {}", e))
                        .await?;
                }
            }
        }
    };

    Ok(())
}

async fn send_help_message(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await
        .map(|_| ()) // yeah idk why I need this
}

async fn handle_pinterest_sticker_set(bot: &Bot, user_id: UserId, url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Download the Pinterest board
    download_board(url)?;

    // Get the image paths
    let image_paths = get_image_paths(url)?;

    // Get the sticker set name
    let (username, boardname) = get_pinterest_info(url)?;
    let sticker_set_name = get_sticker_set_name(bot, &username, &boardname).await?;

    // Create the sticker set
    create_sticker_set(bot, user_id, &sticker_set_name, &image_paths).await?;

    let sticker_set_url = format!("https://t.me/addstickers/{}", sticker_set_name);
    bot.send_message(user_id, format!("Sticker set created: {}", sticker_set_url)).await?;

    Ok(())
}

async fn check_sticker_set_exists(bot: &Bot, name: &str) -> Result<bool, teloxide::errors::ApiError> {
    match bot.get_sticker_set(name).await {
        Ok(_) => Ok(true),  // Sticker set exists
        Err(e) if e.kind() == ApiErrorKind::StickerSetNameInvalid => Ok(false),  // Sticker set doesn't exist
        Err(e) => Err(e),  // Other errors
    }
}

// TODO: Check if support for non-PNG images works
// TODO: Add a size, format and dimensions check for the images
async fn create_sticker(bot: &Bot, path: &str) -> Result<InputSticker, Box<dyn Error + Send + Sync>> {
    if !Path::new(path).exists() {
        return Err("Sticker file not found".into());
    }

    // Open the image file
    let image = ImageReader::open(path)?.decode()?;
    let resized = image.resize_exact(MAX_STICKER_SIZE, MAX_STICKER_SIZE, image::imageops::FilterType::Nearest);

    // Convert the image to PNG and store it in a Cursor
    let mut image_data = Cursor::new(Vec::new());
    resized.write_to(&mut image_data, image::ImageOutputFormat::Png)?;

    // Create an InputFile from the image data
    let input_file = InputFile::memory(image_data.into_inner());

    // Upload the sticker file to Telegram
    let uploaded_file = bot.upload_sticker_file(user_id, input_file).await?;

    // Create an InputSticker from the uploaded file
    let input_sticker = InputSticker::Png(InputFile::file_id(uploaded_file.id));

    Ok(input_sticker)
}

async fn get_sticker_set_name(bot: &Bot, username: &str, boardname: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let botname = bot.get_me().await?.user.username.unwrap_or_default();

    // Iterate over sticker set names until we find one that doesn't exist
    let mut counter = 1;
    loop {
        let sticker_set_name = if counter > 1 {
            format!("{}_{}_{}_by_{}", username, boardname, counter, botname)
        } else {
            format!("{}_{}_by_{}", username, boardname, botname)
        };
    
        let exists = check_sticker_set_exists(bot, &sticker_set_name).await?;
        if !exists {
            println!("Sticker set `{}` does not exist, creating..", sticker_set_name);
            return Ok(sticker_set_name);
        }
        println!("Sticker set `{}` exists already, incrementing..", sticker_set_name);
        counter += 1;
    }
}

// TODO: Add an option to not include the user and board name in the sticker set name for privacy reasons
async fn create_sticker_set(bot: &Bot, user_id: UserId, sticker_set_name: &String, image_paths: &[String]) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create a vector of stickers
    let mut stickers = Vec::new();
    for path in image_paths {
        let sticker = create_sticker(bot, path).await?;
        stickers.push(sticker);
    }

    // Create a new sticker set
    bot.create_new_sticker_set(
        user_id,                   // User ID
        sticker_set_name.clone(),  // Sticker set name
        sticker_set_name.clone(),  // Sticker set title
        stickers[0].clone(),       // Sticker (first sticker in the set?)
        get_random_emoji()         // Emoji 
    ).await?;

    for sticker in stickers.iter().skip(1) {
        let emojis = get_random_emoji(); // Generate a random emoji for each sticker
        bot.add_sticker_to_set(user_id, sticker_set_name.clone(), sticker.clone(), emojis).await?;
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