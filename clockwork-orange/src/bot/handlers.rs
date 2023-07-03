use std::fmt::Debug;

use clockwork_orange_messages::tg_escape;
use color_eyre::{
    eyre::{eyre, Context},
    Result,
};
use log::info;
use teloxide::{
    requests::Requester,
    types::{CallbackQuery, ChatAction, Me, MediaText, Message, MessageKind, Update, User},
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

    info!("Got command {command:?} from @{author}");

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
            .await
            .wrap_err("Failed to send welcome message in /start handler")?;
        }
        Command::AllMy => {
            let items = storage.get_user_items(&author).await?;

            if items.is_empty() {
                bot.send_message(chat_id, "You have no entries yet")
                    .await
                    .wrap_err("Failed to send message in /all_my handler")?;
                return Ok(());
            }

            for (key, item) in items.iter() {
                bot.send_chat_action(chat_id, ChatAction::Typing)
                    .await
                    .wrap_err("Failed to send chat action in /all_my handler")?;
                send_item_to_chat(&bot, item, key, chat_id)
                    .await
                    .wrap_err("Failed to send item in /all_my handler")?;

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }

            bot.send_message(chat_id, tg_escape("That's all!"))
                .await
                .wrap_err("Failed to send finalizing message in /all_my handler")?;
        }
        Command::Random => {
            bot.send_chat_action(chat_id, ChatAction::Typing)
                .await
                .wrap_err("Failed to send chat action in /random handler")?;

            let item = storage
                .get_random()
                .await
                .wrap_err("Failed to get random item in /random handler")?;

            match item {
                Some((key, item)) => {
                    send_item_to_chat(&bot, &item, &key, chat_id)
                        .await
                        .wrap_err("Failed to send item in /random handler")?;
                }
                None => {
                    bot.send_message(chat_id, "You have no entries yet")
                        .await
                        .wrap_err("Failed to send message about empty queue in /random handler")?;
                }
            }
        }
        Command::Unread => {
            let items = storage.get_all().await?;

            if items.is_empty() {
                bot.send_message(chat_id, "You have no entries yet")
                    .await
                    .wrap_err("Failed to send message about empty queue in /unread handler")?;
                return Ok(());
            }

            for (key, item) in items.iter() {
                bot.send_chat_action(chat_id, ChatAction::Typing)
                    .await
                    .wrap_err("Failed to send chat action in /unread handler")?;
                send_item_to_chat(&bot, item, key, chat_id)
                    .await
                    .wrap_err("Failed to send item in /unread handler")?;

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }

            bot.send_message(chat_id, tg_escape("That's all!"))
                .await
                .wrap_err("Failed to send finalizing message in /unread handler")?;
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
            storage
                .mark_as_read(&key)
                .await
                .wrap_err("Marking as read failed")?;

            bot.send_message(chat_id, tg_escape("Great! I hope you liked it 😊"))
                .await
                .wrap_err(
                    "Failed to notify user that we've got \"watched\" item status from him",
                )?;
        }
    }

    bot.answer_callback_query(&callback_query.id)
        .await
        .wrap_err("Failed to set callback answered in TG API")?;

    Ok(())
}

pub async fn add_new_entry<B: StorageBackend>(
    bot: Bot,
    me: Me,
    mut storage: Storage<B>,
    msg: Message,
    author: User,
    text: MediaText,
) -> Result<()> {
    let chat_id = msg.chat.id;
    let author = author.username.unwrap_or_else(|| author.id.to_string());

    // check if user is replying to someone else's message
    match msg.reply_to_message() {
        None => {}
        Some(msg) => {
            if let MessageKind::Common(msg) = &msg.kind {
                if let Some(user) = &msg.from {
                    if user.id != me.id {
                        info!("User is replying to someone else's message, ignoring");

                        return Ok(());
                    }
                }
            }
        }
    }

    let content_item = ContentItem::new(author, text.text);
    storage
        .set(&msg.id.0.to_string().into(), content_item)
        .await
        .wrap_err("Failed to save new item from user")?;

    bot.send_message(chat_id, tg_escape("Saved! 🎉"))
        .await
        .wrap_err("Failed to send confirmation message")?;

    Ok(())
}
