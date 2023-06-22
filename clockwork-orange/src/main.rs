use color_eyre::eyre::WrapErr;
use log::{debug, info};
use teloxide::{
    error_handlers::LoggingErrorHandler,
    update_listeners::{
        polling_default,
        webhooks::{self, Options},
    },
};

use crate::{
    config::{BotMode, Config, StorageKind},
    storage::{MemoryStorage, RedisStorage},
};

mod bot;
mod config;
mod content_item;
mod storage;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    pretty_env_logger::init();
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let (bot, mut dispatcher) = match config.storage {
        StorageKind::InMemory => {
            info!("Creating bot with in-memory storage");
            let storage = MemoryStorage::new().into_storage();
            bot::create_bot_and_dispatcher(storage, &config).await?
        }
        StorageKind::Redis => {
            info!("Creating bot with redis storage");
            let redis_url = config.redis_url.as_ref().expect("REDIS_URL unspecified");
            let storage = RedisStorage::new(redis_url).await?.into_storage();

            bot::create_bot_and_dispatcher(storage, &config).await?
        }
    };
    debug!("Bot created");

    let error_handler = LoggingErrorHandler::new();

    match config.bot_mode {
        BotMode::Polling => {
            info!("Starting bot in polling mode");
            let listener = polling_default(bot).await;

            dispatcher
                .dispatch_with_listener(listener, error_handler)
                .await;
        }
        BotMode::Webhook => {
            info!("Starting bot in webhook mode");
            let listener = webhooks::axum(
                bot,
                Options::new(
                    // FIXME: specify this in config
                    config.bind_to,
                    config.webhook_url.clone(),
                ),
            )
            .await
            .wrap_err("Failed to create webhook listener")?;

            dispatcher
                .dispatch_with_listener(listener, error_handler)
                .await;
        }
    }

    Ok(())
}
