use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

use crate::handlers::*;
use crate::util::env_or_default;

pub(crate) mod handlers;
pub(crate) mod observability;
pub(crate) mod util;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv::dotenv().ok();
    observability::tracing::init_tracer();

    log::info!("Starting sticker exporter bot...");

    let bot = Bot::from_env().set_api_url(
        reqwest::Url::parse(
            env_or_default("TELEGRAM_API_URL", "https://api.telegram.org").as_str(),
        )
        .unwrap(),
    );

    let handler = dptree::entry().branch(
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .filter(|message: &Message| message.chat.is_private()) // only handle private messages
            .branch(
                dptree::case![State::Start]
                    .filter_command::<BasicCommand>()
                    .branch(dptree::case![BasicCommand::Start].endpoint(handle_start))
                    .branch(
                        dptree::case![BasicCommand::SingleExport].endpoint(handle_single_export),
                    )
                    .branch(dptree::case![BasicCommand::PackExport].endpoint(handle_pack_export)),
            )
            .branch(
                dptree::case![State::SingleExport]
                    .filter(|message: Message| message.sticker().is_some())
                    .endpoint(handle_export_sticker),
            )
            .branch(
                dptree::case![State::PackExport]
                    .filter(|message: Message| message.sticker().is_some())
                    .endpoint(handle_export_sticker),
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
