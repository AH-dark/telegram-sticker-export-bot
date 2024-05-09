use std::io::Write;

use anyhow::Context;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};
use teloxide::utils::command::BotCommands;
use zip::ZipWriter;

use crate::util::export_single_sticker;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    SingleExport,
    PackExport,
}

#[derive(Clone, Debug, BotCommands)]
#[command(rename_rule = "lowercase", description = "Basic commands")]
pub enum BasicCommand {
    #[command(description = "Display a brief introduction to the bot")]
    Start,
    #[command(rename = "single", description = "Start single sticker export mode")]
    SingleExport,
    #[command(rename = "pack", description = "Start pack export mode")]
    PackExport,
}

/// Handle the `/start` command, which provides the user with a brief introduction to the bot.
#[tracing::instrument]
pub async fn handle_start(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(
        msg.chat.id,
        r#"
        I am a sticker export bot that can help you export a sticker or an entire sticker pack.
        You can use the following commands to enter different modes:

        /single - Export a single sticker
        /pack - Export an entire sticker pack

        You can also use the /cancel command to cancel the current operation.

        This bot is open source. You can find the source code on <a href="https://github.com/AH-dark/telegram-sticker-export-bot">AH-dark/telegram-sticker-export-bot</a>. If you have any questions or suggestions, please feel free to open an issue or pull request. If you like this bot, please give it a star. Thank you!
        "#.split('\n').map(|s| s.trim()).collect::<Vec<_>>().join("\n"),
    )
        .parse_mode(ParseMode::Html)
        .send()
        .await?;

    Ok(())
}

/// Handle the `/cancel` command, which allows the user to cancel the current operation.
#[tracing::instrument]
pub async fn handle_cancel(
    bot: Bot,
    update: Update,
    dialogue: Dialogue<State, InMemStorage<State>>,
) -> anyhow::Result<()> {
    let chat = match update.chat() {
        Some(chat) => chat,
        None => return Err(anyhow::anyhow!("No chat found in the update")),
    };

    // Reset the dialogue state
    dialogue
        .reset()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to reset dialogue: {}", e))?;

    bot.send_message(chat.id, "Operation canceled.")
        .send()
        .await?;

    Ok(())
}

/// Handle the `/single` command, which allows the user to export a single sticker.
#[tracing::instrument]
pub async fn handle_single_export(
    bot: Bot,
    message: Message,
    dialogue: Dialogue<State, InMemStorage<State>>,
) -> anyhow::Result<()> {
    // Update the dialogue state
    dialogue
        .update(State::SingleExport)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update state: {}", e))?;

    // Reply to the user
    bot.send_message(
        message.chat.id,
        "Single export mode, please send me stickers.",
    )
    .reply_to_message_id(message.id)
    .send()
    .await?;

    Ok(())
}

/// Handle the `/pack` command, which allows the user to export a single sticker.
#[tracing::instrument]
pub async fn handle_pack_export(
    bot: Bot,
    message: Message,
    dialogue: Dialogue<State, InMemStorage<State>>,
) -> anyhow::Result<()> {
    // Update the dialogue state
    dialogue
        .update(State::PackExport)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update state: {}", e))?;

    // Reply to the user
    bot.send_message(
        message.chat.id,
        "Pack export mode, please send me stickers from the sticker pack you want to export.",
    )
    .reply_to_message_id(message.id)
    .send()
    .await?;

    Ok(())
}

/// Handle the `/single` and `/pack` commands, which allow the user to export a single sticker or an entire sticker pack.
#[tracing::instrument]
pub async fn handle_export_sticker(
    bot: Bot,
    message: Message,
    dialogue: Dialogue<State, InMemStorage<State>>,
) -> anyhow::Result<()> {
    let waiting_msg = bot
        .send_message(message.chat.id, "Processing...")
        .reply_to_message_id(message.id)
        .send()
        .await?;

    // Check if the message contains a sticker
    let sticker = match message.sticker() {
        Some(sticker) => sticker,
        None => {
            bot.send_message(message.chat.id, "Please send me a sticker.")
                .reply_to_message_id(message.id)
                .send()
                .await?;

            bot.delete_message(message.chat.id, waiting_msg.id)
                .send()
                .await?;

            return Ok(());
        }
    };

    match dialogue.get_or_default().await {
        Ok(State::SingleExport) => {
            match export_single_sticker(bot.clone(), sticker).await {
                Ok((filename, data)) => {
                    bot.send_document(message.chat.id, InputFile::memory(data).file_name(filename))
                        .reply_to_message_id(message.id)
                        .send()
                        .await?;
                }
                Err(e) => {
                    bot.send_message(message.chat.id, e.to_string())
                        .reply_to_message_id(message.id)
                        .send()
                        .await?;

                    bot.delete_message(message.chat.id, waiting_msg.id)
                        .send()
                        .await?;

                    return Err(e);
                }
            };
        }
        Ok(State::PackExport) => {
            // check if the sticker is from a sticker pack
            if sticker.set_name.is_none() {
                bot.send_message(
                    message.chat.id,
                    "Please send me a sticker from a sticker pack.",
                )
                .reply_to_message_id(message.id)
                .send()
                .await?;

                bot.delete_message(message.chat.id, waiting_msg.id)
                    .send()
                    .await?;

                return Err(anyhow::anyhow!("Invalid sticker pack"));
            }

            // Get the sticker set
            let sticker_set = bot
                .get_sticker_set(sticker.set_name.as_ref().unwrap())
                .await
                .context("Failed to get sticker set")?;

            // Get the stickers in the sticker pack
            let mut futures = FuturesUnordered::new();
            let stickers_len = sticker_set.stickers.len();

            for sticker in sticker_set.stickers {
                let bot = bot.clone();
                futures.push(async move { export_single_sticker(bot, &sticker).await });
            }

            let mut sticker_files = Vec::new();
            let mut downloaded_len = 0;

            while let Some(result) = futures.next().await {
                match result {
                    Ok((filename, data)) => {
                        sticker_files.push((filename, data));
                        downloaded_len += 1;

                        // Update progress every 5 stickers
                        if downloaded_len % 5 == 0 || downloaded_len == stickers_len {
                            bot.edit_message_text(
                                message.chat.id,
                                waiting_msg.id,
                                format!("Downloading... {}/{}", downloaded_len, stickers_len),
                            )
                            .send()
                            .await?;
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to export sticker: {}", e);

                        bot.send_message(message.chat.id, e.to_string())
                            .reply_to_message_id(message.id)
                            .send()
                            .await?;

                        bot.delete_message(message.chat.id, waiting_msg.id)
                            .send()
                            .await?;

                        return Err(e);
                    }
                }
            }

            // Create a zip archive containing all the stickers
            let mut buffer = Vec::new();
            {
                let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
                let options: zip::write::FileOptions<zip::write::ExtendedFileOptions> =
                    zip::write::FileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated)
                        .unix_permissions(0o755);

                for (i, (filename, data)) in sticker_files.iter().enumerate() {
                    zip.start_file(filename.to_owned(), options.clone())
                        .context("Failed to start file in zip archive")?;
                    zip.write_all(&data)
                        .context("Failed to write file to zip archive")?;

                    // update status
                    if i % 5 == 0 || i == stickers_len {
                        bot.edit_message_text(
                            message.chat.id,
                            waiting_msg.id,
                            format!("Compressing... {}/{}", &i, &stickers_len),
                        )
                        .send()
                        .await
                        .context("Failed to update status")
                        .ok();
                    }
                }

                zip.finish().context("Failed to finish zip archive")?;
            }

            // update status
            bot.edit_message_text(message.chat.id, waiting_msg.id, "Uploading zip archive...")
                .send()
                .await?;

            bot.send_document(
                message.chat.id,
                InputFile::memory(buffer).file_name(format!("stickers-{}.zip", &sticker_set.name)),
            )
            .reply_to_message_id(message.id)
            .send()
            .await?;
        }
        Ok(_) => {
            unreachable!("Invalid state")
        }
        Err(e) => {
            bot.send_message(
                message.chat.id,
                format!("Failed to get state, state manager error: {}", e),
            )
            .reply_to_message_id(message.id)
            .send()
            .await?;
        }
    }

    bot.delete_message(message.chat.id, waiting_msg.id)
        .send()
        .await?;

    Ok(())
}
