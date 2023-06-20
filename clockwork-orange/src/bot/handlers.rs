use std::fmt::Debug;

use clockwork_orange_messages::tg_escape;
use color_eyre::{eyre::eyre, Result};
use teloxide::{
    requests::Requester,
    types::{CallbackQuery, ChatAction, MediaText, Message, Update, User},
};

use crate::{
    content_item::ContentItem,
    storage::{Storage, StorageBackend},
};

use super::{callbacks::Callback, send_item_to_chat, Bot, Command};

pub async fn handle_command<B: StorageBackend + Debug>(
    bot: Bot,
    storage: Storage<B>,
    msg: Message,
    author: User,
    command: Command,
) -> Result<()> {
    let chat_id = msg.chat.id;
    let author = author
        .username
        .clone()
        .unwrap_or_else(|| author.id.to_string());

    match command {
        Command::Start => {
            let bot_for_line = match author.as_str() {
                "utterstep" => "you and @anna_kha",
                "anna_kha" => "you and @utterstep (btw, he loves you)",
                _ => "@utterstep and @anna_kha",
            };
            let whose = match author.as_str() {
                "utterstep" => "your",
                "anna_kha" => "your",
                _ => "their",
            };

            bot.send_message(
                chat_id,
                tg_escape(&format!(
                    indoc::indoc! {"
                    Hello, @{}! I'm a bot for {} to keep {} watch list.
                    \nJust send me anything and I'll add it to the list!"},
                    author, bot_for_line, whose
                )),
            )
            .await?;
        }
        Command::AllMy => {
            let items = storage.get_user_items(&author).await?;

            if items.is_empty() {
                bot.send_message(chat_id, "You have no entries yet").await?;
                return Ok(());
            }

            for (key, item) in items.iter() {
                bot.send_chat_action(chat_id, ChatAction::Typing).await?;
                send_item_to_chat(&bot, item, key, chat_id).await?;

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }

            bot.send_message(chat_id, tg_escape("That's all!")).await?;
        }
        Command::Random => {
            bot.send_chat_action(chat_id, ChatAction::Typing).await?;

            match storage.get_random().await {
                Ok(Some((key, item))) => {
                    send_item_to_chat(&bot, &item, &key, chat_id).await?;
                }
                Ok(None) => {
                    bot.send_message(chat_id, "You have no entries yet").await?;
                }
                Err(e) => {
                    bot.send_message(
                        chat_id,
                        tg_escape(&format!("Some error happened :(\n\ndebug info:\n```{e}```")),
                    )
                    .await?;
                }
            }
        }
        Command::Unread => {
            let items = storage.get_all().await?;

            if items.is_empty() {
                bot.send_message(chat_id, "You have no entries yet").await?;
                return Ok(());
            }

            for (key, item) in items.iter() {
                bot.send_chat_action(chat_id, ChatAction::Typing).await?;
                send_item_to_chat(&bot, item, key, chat_id).await?;

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }

            bot.send_message(chat_id, tg_escape("That's all!")).await?;
        }
    }

    Ok(())
}

pub async fn handle_callback<B: StorageBackend>(
    bot: Bot,
    mut storage: Storage<B>,
    update: Update,
    callback_query: CallbackQuery,
    callback: Callback,
) -> Result<()> {
    let chat_id = update.chat().ok_or_else(|| eyre!("No chat in update"))?.id;

    match callback {
        Callback::MarkAsRead(key) => {
            storage.mark_as_read(&key).await?;

            bot.send_message(chat_id, tg_escape("Marked as read! I hope you liked it 😊"))
                .await?;
        }
    }

    bot.answer_callback_query(&callback_query.id).await?;

    Ok(())
}

pub async fn add_new_entry<B: StorageBackend>(
    bot: Bot,
    mut storage: Storage<B>,
    msg: Message,
    author: User,
    text: MediaText,
) -> Result<()> {
    let chat_id = msg.chat.id;
    let author = author.username.unwrap_or_else(|| author.id.to_string());

    let content_item = ContentItem::new(author, text.text);
    storage
        .set(&msg.id.0.to_string().into(), content_item)
        .await?;

    bot.send_message(chat_id, tg_escape("Saved! 🎉")).await?;

    Ok(())
}
