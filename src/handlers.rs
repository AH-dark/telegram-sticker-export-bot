use infer::Infer;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use teloxide::types::{InputFile, ParseMode};
use teloxide::utils::command::BotCommands;

use crate::util::{convert_unknown_image_to_png, convert_webm_to_gif};

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
                    bot.send_message(message.chat.id, "Failed to infer the file type.")
                        .reply_to_message_id(message.id)
                        .send()
                        .await?;

                    bot.delete_message(message.chat.id, waiting_msg.id)
                        .send()
                        .await?;

                    return Ok(());
                }
            };

            // handle the file type
            let mime = kind.mime_type();
            match mime.split('/').next().unwrap_or_default() {
                "image" => {
                    let data = match convert_unknown_image_to_png(&file_data) {
                        Ok(data) => data,
                        Err(e) => {
                            bot.send_message(message.chat.id, e.to_string())
                                .reply_to_message_id(message.id)
                                .send()
                                .await?;

                            bot.delete_message(message.chat.id, waiting_msg.id)
                                .send()
                                .await?;

                            return Ok(());
                        }
                    };

                    bot.send_document(
                        message.chat.id,
                        InputFile::memory(data)
                            .file_name(format!("{}.png", sticker.file.unique_id)),
                    )
                    .reply_to_message_id(message.id)
                    .send()
                    .await?;
                }
                "video" => {
                    let data = convert_webm_to_gif(&file_data).await?;

                    bot.send_document(
                        message.chat.id,
                        InputFile::memory(data)
                            .file_name(format!("{}.gif", sticker.file.unique_id)),
                    )
                    .reply_to_message_id(message.id)
                    .send()
                    .await?;
                }
                _ => {
                    bot.send_message(message.chat.id, format!("Unsupported file type: {}", mime))
                        .reply_to_message_id(message.id)
                        .send()
                        .await?;
                }
            }
        }
        Ok(State::PackExport) => {
            todo!("Export sticker pack")
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
