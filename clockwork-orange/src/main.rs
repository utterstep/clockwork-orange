use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry, EnvFilter};
use tracing_tree::HierarchicalLayer;

use crate::{
    config::{BotMode, Config, StorageKind},
    storage::{MemoryStorage, RedisStorage},
};

mod bot;
mod config;
mod content_item;
mod listeners;
mod storage;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let layer = HierarchicalLayer::default()
        .with_indent_lines(true)
        .with_bracketed_fields(true)
        .with_thread_ids(true);

    let subscriber = Registry::default()
        .with(layer)
        .with(EnvFilter::from_default_env());
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let config = Config::from_env()?;

    match config.storage {
        StorageKind::InMemory => {
            info!("Creating bot with in-memory storage");
            let storage = MemoryStorage::new().into_storage();

            let (bot, dispatcher) =
                bot::create_bot_and_dispatcher(storage.clone(), &config).await?;

            match config.bot_mode {
                BotMode::Polling => {
                    listeners::start_polling(dispatcher, bot).await?;
                }
                BotMode::Webhook => {
                    listeners::start_webhook(dispatcher, bot, storage, &config).await?;
                }
            }
        }
        StorageKind::Redis => {
            info!("Creating bot with redis storage");
            let redis_url = config.redis_url.as_ref().expect("REDIS_URL unspecified");
            let storage = RedisStorage::new(redis_url).await?.into_storage();

            let (bot, dispatcher) =
                bot::create_bot_and_dispatcher(storage.clone(), &config).await?;

            match config.bot_mode {
                BotMode::Polling => {
                    listeners::start_polling(dispatcher, bot).await?;
                }
                BotMode::Webhook => {
                    listeners::start_webhook(dispatcher, bot, storage, &config).await?;
                }
            }
        }
    };
    debug!("Bot created");

    Ok(())
}
