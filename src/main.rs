use std::sync::Arc;

use governor::{clock, Quota};
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

use crate::handlers::*;
use crate::limiter::Limiter;
use crate::util::env_or_default;

pub(crate) mod handlers;
pub(crate) mod limiter;
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

    let clock = clock::DefaultClock::default();
    let rate_limiter: Arc<Limiter<i64>> = Limiter::new(
        Quota::per_minute(
            std::env::var("RATE_LIMIT")
                .unwrap_or("20".to_string())
                .parse()
                .unwrap(),
        )
        .allow_burst(
            std::env::var("RATE_LIMIT_BURST")
                .unwrap_or("5".to_string())
                .parse()
                .unwrap(),
        ),
        &clock,
    );

    Dispatcher::builder(
        bot,
        dptree::entry().branch(
            Update::filter_message()
                .enter_dialogue::<Message, InMemStorage<State>, State>()
                .filter(|message: Message| message.chat.is_private()) // only handle private messages
                .branch(
                    dptree::case![State::Start]
                        .filter_command::<BasicCommand>()
                        .branch(dptree::case![BasicCommand::Start].endpoint(handle_start))
                        .branch(dptree::case![BasicCommand::Help].endpoint(handle_help))
                        .branch(
                            dptree::case![BasicCommand::SingleExport]
                                .endpoint(handle_single_export),
                        )
                        .branch(
                            dptree::case![BasicCommand::PackExport].endpoint(handle_pack_export),
                        ),
                )
                .branch(dptree::case![State::SingleExport].endpoint(handle_export_sticker))
                .branch(dptree::case![State::PackExport].endpoint(handle_export_sticker))
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
        ),
    )
    .distribution_function(|_| None::<std::convert::Infallible>)
    .dependencies(dptree::deps![InMemStorage::<State>::new(), rate_limiter])
    .build()
    .dispatch()
    .await;
}
