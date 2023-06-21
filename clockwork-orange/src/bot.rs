use std::fmt::Debug;

use color_eyre::{eyre::WrapErr, Report, Result};
use teloxide::{
    adaptors::DefaultParseMode,
    dispatching::{DefaultKey, HandlerExt, UpdateFilterExt},
    payloads::SendMessageSetters,
    prelude::Dispatcher as TgDispatcher,
    requests::{Requester, RequesterExt},
    types::{ChatId, InlineKeyboardMarkup, ParseMode, Update},
    utils::command::BotCommands,
    Bot as TgBot,
};

use crate::{
    config::Config,
    content_item::ContentItem,
    storage::{Key, Storage, StorageBackend},
};

mod callbacks;
use callbacks::Callback;

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
        // generic Callback handler
        .branch(
            Update::filter_callback_query()
                .filter_map(extractors::get_callback_data)
                .endpoint(handlers::handle_callback::<B>),
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

/// Send a message to chat, with a button to mark the item as watched
pub(self) async fn send_item_to_chat<R>(
    requester: R,
    item: &ContentItem,
    key: &Key,
    chat_id: ChatId,
) -> Result<()>
where
    R: Requester + Send + Sync,
    <R as Requester>::Err: Send + Sync + 'static,
{
    let message_text = item.to_tg_message_text();

    requester
        .send_message(chat_id, &message_text)
        .reply_markup(InlineKeyboardMarkup::new(vec![vec![
            Callback::mark_as_read(key).as_button("☑️ Mark as watched"),
        ]]))
        .await
        .wrap_err_with(|| format!("Failed to send a message to chat, message: {message_text}"))?;

    Ok(())
}
