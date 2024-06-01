use std::fs;
use std::path::Path;
use std::process::Command as SystemCommand;
use std::error::Error;

use chrono::Utc;
use serde_json::Value;
use rand::seq::SliceRandom;
use image::{DynamicImage, GenericImageView, ImageError, ImageDecoder, imageops::FilterType, io::Reader as ImageReader};
use teloxide::{prelude::*, types::{InputFile, UserId, InputSticker, Message}, utils::command::BotCommands};
use urlencoding::decode;
use log::{trace, debug, info, warn, error};
use image::codecs::png::PngEncoder;
use oxipng::Options;
use image::ImageEncoder;
use image::ColorType;
use std::io::Cursor;

const STICKER_SIZE: u32 = 512;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting sticker bot...");

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

// Handles the commands
async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            handle_help_command(bot, msg).await?
        }
        Command::CreateSet(url) => {
            match handle_create_set(&bot, msg.from().unwrap().id, &url).await {
                Ok(_) => (),
                Err(e) => {
                    // TODO: Maybe don't send the error message to the user, but log it instead
                    error!("Failed to handle Pinterest sticker set: {}", e);
                    bot.send_message(msg.chat.id, format!("Failed to create sticker set: {}", e))
                        .await?;
                }
            }
        }
    };
    Ok(())
}

async fn handle_help_command(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await
        .map(|_| ()) // yeah idk why i need this, i'll figure out later i guess
}

async fn handle_create_set(bot: &Bot, user_id: UserId, url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
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



pub struct Img {
    img : DynamicImage,
    path: String,
}

impl Img {
    fn new(path: String) -> Result<Self, ImageError> {
        trace!("Opening image: {}", path);
        trace!("Image size: {:.2} KB", fs::metadata(&path).unwrap().len() as f64 / 1024.0);
        let img = ImageReader::open(&path)?.decode()?;
        Ok(Self { img, path })
    }


    
    async fn encode_png(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut img = self.img.clone();
    
        // Ensure the image is a square with dimensions STICKER_SIZE x STICKER_SIZE
        if img.width() != STICKER_SIZE || img.height() != STICKER_SIZE {
            img = img.resize_exact(STICKER_SIZE, STICKER_SIZE, FilterType::Gaussian);
        }
    
        let mut buffer = Cursor::new(Vec::new());
        let encoder = PngEncoder::new(&mut buffer);
        let img_data = img.to_rgba8().into_raw();
        encoder.write_image(
            &img_data, 
            img.width(), 
            img.height(), 
            ColorType::Rgba8.into(),
        )?;
    
        Ok(buffer.into_inner())
    }
}



async fn create_sticker(bot: &Bot, path: &str, user_id: UserId) -> Result<InputSticker, Box<dyn Error + Send + Sync>> {
    debug!("Creating sticker from file: {}", path);

    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Err("Sticker file not found".into());
    }

    // Check if the file size is greater than 512KB
    let metadata = fs::metadata(&path_obj)?;
    let file_size_kb = metadata.len() as f64 / 1024.0;
    if file_size_kb > 512.0 {
        return Err("Image size is greater than 512KB".into());
    }

    let image = Img::new(path.to_string())?;
    let options = oxipng::Options::default();
    let optimized_data = oxipng::optimize_from_memory(&image.encode_png().await?, &options)?;
    
    trace!("Optimized data size: {:.2} KB", optimized_data.len() as f64 / 1024.0);

    let input_file    = InputFile::memory(optimized_data);                       // Create an InputFile from the image data
    let uploaded_file = bot.upload_sticker_file(user_id, input_file).await?;     // Upload the sticker file to Telegram
    let input_sticker = InputSticker::Png(InputFile::file_id(uploaded_file.id)); // Create an InputSticker from the uploaded file

    Ok(input_sticker)
}


async fn get_sticker_set_name(bot: &Bot, username: &str, boardname: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let botname = bot.get_me().await?.user.username.unwrap_or_default();

    // Get the current timestamp in milliseconds
    let timestamp = Utc::now().timestamp_millis();

    // Convert the timestamp to a string
    let timestamp_str = timestamp.to_string();

    // Get the last three characters of the timestamp string
    let last_three_digits = &timestamp_str[timestamp_str.len() - 3..];

    // Include the last three digits of the timestamp in the sticker set name
    let sticker_set_name = format!("{}_{}_{}_by_{}", username, boardname, last_three_digits, botname);

    Ok(sticker_set_name)
}

// TODO: Add an option to not include the user and board name in the sticker set name for privacy reasons
async fn create_sticker_set(bot: &Bot, user_id: UserId, sticker_set_name: &String, image_paths: &[String]) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create a vector of stickers
    let mut stickers = Vec::new();
    for path in image_paths {
        match create_sticker(bot, path, user_id).await {
            Ok(sticker) => stickers.push(sticker),
            Err(e) => {
                error!("Failed to create sticker from file {}: {}", path, e);
                continue;
            }
        }
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

    let url = decode(url).expect("UTF-8");

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

    let dir_path = Path::new("gallery-dl").join("pinterest").join(username).join(board_name);
    debug!("Directory path: {}", dir_path.display()); // Print the directory path

    if !dir_path.is_dir() {
        return Err(format!("Invalid directory path: {}", dir_path.display()).into()); // Include the directory path in the error message
    }

    let mut paths = Vec::new();

    for entry in fs::read_dir(&dir_path)? {
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