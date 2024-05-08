use std::io::Cursor;
use std::process::Command;

use anyhow::Context;
use image::ImageFormat;
use image::io::Reader as ImageReader;
use tokio::fs;

/// Get the value of an environment variable or a default value.
#[tracing::instrument]
pub fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
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
