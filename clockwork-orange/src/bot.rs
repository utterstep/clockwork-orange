use std::fmt::Debug;

use color_eyre::{Report, Result};
use teloxide::{
    adaptors::DefaultParseMode,
    dispatching::{DefaultKey, HandlerExt, UpdateFilterExt},
    prelude::Dispatcher as TgDispatcher,
    requests::{Requester, RequesterExt},
    types::{ParseMode, Update},
    utils::command::BotCommands,
    Bot as TgBot,
};

use crate::{
    config::Config,
    storage::{Storage, StorageBackend},
};

mod commands;
use commands::Command;

mod extractors;

mod handlers;
use handlers::{add_new_entry, handle_command};

pub type Bot = DefaultParseMode<TgBot>;
pub type Dispatcher<'a> = TgDispatcher<Bot, Report, DefaultKey>;

pub async fn create_bot() -> Result<Bot> {
    let bot = TgBot::from_env().parse_mode(ParseMode::MarkdownV2);
    bot.set_my_commands(Command::bot_commands()).await?;

    Ok(bot)
}

pub async fn create_bot_and_dispatcher<B: StorageBackend + Debug + 'static>(
    storage: Storage<B>,
    config: &Config,
) -> Result<(Bot, Dispatcher)> {
    let bot = create_bot().await?;

    let handler = dptree::entry()
        // generic Command handler
        .branch(
            Update::filter_message().chain(
                dptree::entry()
                    .filter_command::<Command>()
                    .filter_map(extractors::get_message_author)
                    .endpoint(handle_command::<B>),
            ),
        )
        // any other text message – append to diary
        .branch(
            Update::filter_message()
                .filter_map(extractors::get_message_text)
                .filter_map(extractors::get_message_author)
                .endpoint(add_new_entry::<B>),
        );

    Ok((
        bot.clone(),
        TgDispatcher::builder(bot, handler)
            .dependencies(dptree::deps![storage, config.clone()])
            .enable_ctrlc_handler()
            .build(),
    ))
}
