use std::io::Cursor;

use anyhow::Context;
use image::ImageFormat;
use image::io::Reader as ImageReader;

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
    todo!()
}
