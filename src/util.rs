use std::io::Cursor;
use std::process::Command;

use anyhow::Context;
use image::ImageFormat;
use image::io::Reader as ImageReader;
use infer::Infer;
use teloxide::Bot;
use teloxide::prelude::{Request, Requester};
use teloxide::types::Sticker;
use tokio::fs;

/// Get the value of an environment variable or a default value.
#[tracing::instrument]
pub fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Export a single sticker.
#[tracing::instrument]
pub async fn export_single_sticker(
    bot: &Bot,
    sticker: &Sticker,
) -> anyhow::Result<(String, Vec<u8>)> {
    // download the sticker file
    let file = bot.get_file(sticker.file.id.clone()).send().await?;
    let file_url = format!(
        "{}/file/bot{}/{}",
        bot.api_url().as_str(),
        bot.token(),
        file.path
    );
    let file_data = reqwest::get(file_url).await?.bytes().await?;

    // infer the file type
    let infer = Infer::new();
    let kind = match infer.get(&file_data) {
        Some(t) => t,
        None => {
            return Err(anyhow::anyhow!("Failed to infer file type"));
        }
    };

    // handle the file type
    let mime = kind.mime_type();
    match mime.split('/').next().unwrap_or_default() {
        "image" => {
            let data = match convert_unknown_image_to_png(&file_data) {
                Ok(data) => data,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to convert image to PNG: {}", e));
                }
            };

            Ok((format!("{}.png", sticker.file.unique_id), data))
        }
        "video" => {
            let data = convert_webm_to_gif(&file_data).await?;

            Ok((format!("{}.gif", sticker.file.unique_id), data))
        }
        _ => Err(anyhow::anyhow!("Unsupported file type")),
    }
}

/// Convert an unknown image to PNG format.
#[tracing::instrument]
pub fn convert_unknown_image_to_png(image: &[u8]) -> anyhow::Result<Vec<u8>> {
    let img = ImageReader::new(Cursor::new(image))
        .with_guessed_format()?
        .decode()
        .context("Failed to decode image")?;

    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .context("Failed to encode image")?;

    Ok(buf)
}

/// Convert a webm video to a GIF.
#[tracing::instrument]
pub async fn convert_webm_to_gif(video: &[u8]) -> anyhow::Result<Vec<u8>> {
    let temp_dir = tempfile::tempdir().context("Failed to create a temporary directory")?;
    log::debug!("Temporary directory: {:?}", temp_dir.path());

    let video_path = &temp_dir.path().join("video.webm");
    let gif_path = &temp_dir.path().join("video.gif");

    fs::write(&video_path, video)
        .await
        .context("Failed to write video to disk")?;

    let output = Command::new("ffmpeg")
        .args([
            "-i",
            video_path.to_str().unwrap(),
            "-vf",
            "fps=30,scale=320:-1:flags=lanczos",
            "-c:v",
            "gif",
            "-f",
            "gif",
            gif_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to convert video to GIF")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to convert video to GIF: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let gif = tokio::fs::read(&gif_path)
        .await
        .context("Failed to read GIF from disk")?;

    Ok(gif)
}
