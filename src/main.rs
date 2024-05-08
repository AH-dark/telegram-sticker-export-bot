use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use teloxide::types::{Chat, ParseMode};

use crate::util::env_or_default;

mod observability;
mod util;

#[derive(Clone, Default, Debug)]
enum State {
    #[default]
    Start,
}

#[tracing::instrument]
async fn handle_start(bot: Bot, msg: Message) -> anyhow::Result<()> {
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

#[tracing::instrument]
async fn handle_cancel(
    bot: Bot,
    update: Update,
    dialogue: Dialogue<State, InMemStorage<State>>,
) -> anyhow::Result<()> {
    let chat = match update.chat() {
        Some(chat) => chat,
        None => return Err(anyhow::anyhow!("No chat found in the update")),
    };

    dialogue
        .reset()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to reset dialogue: {}", e))?;

    bot.send_message(chat.id, "Operation canceled.")
        .send()
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv::dotenv().ok();
    observability::tracing::init_tracer();
    log::info!("Starting call the police bot...");

    let bot = Bot::from_env().set_api_url(
        reqwest::Url::parse(
            env_or_default("TELEGRAM_API_URL", "https://api.telegram.org").as_str(),
        )
        .unwrap(),
    );

    let handler = dptree::entry().branch(
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(
                dptree::case![State::Start]
                    .filter(|message: Message| {
                        message.text().map(|text| text == "/start").unwrap_or(false)
                    })
                    .endpoint(handle_start),
            )
            .branch(
                dptree::entry()
                    .filter(|message: Message| {
                        message
                            .text()
                            .map(|text| text == "/cancel")
                            .unwrap_or(false)
                    })
                    .endpoint(handle_cancel),
            ),
    );

    Dispatcher::builder(bot, handler)
        .distribution_function(|_| None::<std::convert::Infallible>)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .build()
        .dispatch()
        .await;
}
