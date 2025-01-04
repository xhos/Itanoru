use crate::stickers::FuncResult;
use log::trace;
use serde_json::Value;
use std::{fs, path::Path, process::Command as SystemCommand};

pub struct PinterestInfo {
  username: String,
  board_name: String,
  section: Option<String>,
}

fn parse_pinterest_url(url: &str) -> Option<PinterestInfo> {
  let parts: Vec<&str> = url.split('/').collect();
  let mut section = None;

  // Skip protocol and domain parts
  let username_idx = parts
    .iter()
    .position(|&p| !p.is_empty() && !p.contains("pinterest") && !p.contains("http"))?;

  let username = parts[username_idx].to_string();
  let board_name = parts.get(username_idx + 1)?.to_string();

  if let Some(section_part) = parts.get(username_idx + 2) {
    if section_part.starts_with("set") {
      section = Some(section_part.to_string());
      trace!("detected board section: {}", section_part);
    }
  }

  trace!(
    "parsed Pinterest URL: user={}, board={}, section={:?}",
    username,
    board_name,
    section
  );

  Some(PinterestInfo {
    username,
    board_name,
    section,
  })
}

pub fn get_pinterest_info(url: &str) -> FuncResult<(String, String, Option<String>)> {
  trace!("fetching Pinterest info: {}", url);
  let output = SystemCommand::new("gallery-dl")
    .arg("-j")
    .arg(url)
    .output()?;

  if !output.status.success() {
    return Err("gallery-dl command failed".into());
  }

  let info = parse_pinterest_url(url).ok_or("Failed to parse URL")?;
  Ok((info.username, info.board_name, info.section))
}

pub fn get_image_count(url: &str) -> FuncResult<usize> {
  let info = parse_pinterest_url(url).ok_or("Failed to parse URL")?;
  trace!(
    "counting images for {}/{}{}",
    info.username,
    info.board_name,
    info
      .section
      .as_ref()
      .map_or("".to_string(), |s| format!("/{}", s))
  );

  let output = SystemCommand::new("gallery-dl")
    .args(&["-j", url])
    .output()?;

  if !output.status.success() {
    return Err("Failed to get image count".into());
  }

  let json: Value = serde_json::from_slice(&output.stdout)?;
  let mut unparseable = Vec::new();

  let count = match &info.section {
    Some(section) => json
      .as_array()
      .map(|arr| {
        arr
          .iter()
          .filter(|item| {
            let section_match = item
              .get(1)
              .and_then(|obj| obj.get("section"))
              .and_then(|s| s.as_str())
              == Some(section);

            if !section_match {
              if let Some(url) = item
                .get(1)
                .and_then(|obj| obj.get("url"))
                .and_then(|u| u.as_str())
              {
                unparseable.push(url.to_string());
              }
            }
            section_match
          })
          .count()
      })
      .unwrap_or(0),
    None => json.as_array().map(|arr| arr.len()).unwrap_or(0),
  };

  trace!(
    "found {} images in {}{}",
    count,
    info.board_name,
    info
      .section
      .as_ref()
      .map_or("".to_string(), |s| format!("/{}", s))
  );

  if !unparseable.is_empty() {
    trace!("Failed to parse {} items:", unparseable.len());
    for url in unparseable {
      trace!("Unparseable item: {}", url);
    }
  }

  Ok(count)
}

pub fn download_board(url: &str) -> FuncResult<()> {
  trace!("downloading board: {}", url);
  fs::create_dir_all("data")?;

  let info = parse_pinterest_url(url).ok_or("Failed to parse URL")?;

  let filename_pattern = match info.section {
    Some(section) => format!(
      "{{board[owner][username]}}_{{board[name]}}_{}_{{id}}.{{extension}}",
      section
    ),
    None => String::from("{board[owner][username]}_{board[name]}_{id}.{extension}"),
  };

  let output = SystemCommand::new("gallery-dl")
    .arg(url)
    .arg("-D")
    .arg("data")
    .arg("-f")
    .arg(filename_pattern)
    .output()?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(format!("gallery-dl failed: {}", stderr).into());
  }

  trace!("download complete");
  Ok(())
}

pub fn get_image_paths(username: &str, boardname: &str) -> FuncResult<Vec<String>> {
  trace!("gathering paths for {}/{}", username, boardname);
  let dir = Path::new("data");
  if !dir.is_dir() {
    return Err("Data directory not found".into());
  }

  let mut paths = Vec::new();
  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    if entry.path().is_file() {
      let fname = entry.file_name().to_string_lossy().to_string();
      if fname.contains(username) && fname.contains(boardname) {
        paths.push(entry.path().to_string_lossy().into_owned());
      }
    }
  }

  trace!("found {} images", paths.len());
  Ok(paths)
}

pub fn cleanup_data(username: &str, boardname: &str) -> FuncResult<()> {
  let dir = Path::new("data");
  if !dir.is_dir() {
    return Ok(());
  }

  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    if entry.path().is_file() {
      let filename = entry.file_name().to_string_lossy().to_string();
      if filename.contains(username) && filename.contains(boardname) {
        fs::remove_file(entry.path())?;
      }
    }
  }
  Ok(())
}
