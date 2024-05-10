use std::io::Cursor;
use std::process::Command;

use anyhow::Context;
use image::ImageFormat;
use image::io::Reader as ImageReader;
use infer::Infer;
use teloxide::Bot;
use teloxide::net::Download;
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
    bot: Bot,
    sticker: &Sticker,
) -> anyhow::Result<(String, Vec<u8>)> {
    // download the sticker file
    let file = bot
        .get_file(sticker.file.id.clone())
        .send()
        .await
        .context("Failed to get file info")?;

    let file_data = if file.path.starts_with('/') {
        // adapted to the local api server
        fs::read(&file.path).await.context("Failed to read file")?
    } else {
        let mut file_data = Vec::new();
        bot.download_file(&file.path, &mut Cursor::new(&mut file_data))
            .await
            .context("Failed to download file")?;
        file_data
    };

    // infer the file type
    let infer = Infer::new();
    let kind = infer.get(&file_data).context("Failed to infer file type")?;

    // handle the file type
    let mime = kind.mime_type();
    match mime.split('/').next().unwrap_or_default() {
        "image" => {
            let data =
                convert_unknown_image_to_png(&file_data).context("Failed to convert image")?;

            Ok((format!("{}.png", sticker.file.unique_id), data))
        }
        "video" => {
            let data = convert_webm_to_gif(&file_data)
                .await
                .context("Failed to convert video")?;

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
