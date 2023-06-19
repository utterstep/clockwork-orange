use std::fmt::Debug;

use color_eyre::Result;
use teloxide::{
    requests::Requester,
    types::{ChatAction, MediaText, Message, User},
};

use clockwork_orange_messages::tg_escape;

use crate::{
    content_item::ContentItem,
    storage::{Storage, StorageBackend},
};

use super::{Bot, Command};

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

            for item in items.values() {
                item.send_to_chat(&bot, chat_id).await?;
                bot.send_chat_action(chat_id, ChatAction::Typing).await?;

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        }
        Command::Random => match storage.get_random().await {
            Ok(Some(item)) => {
                item.send_to_chat(&bot, chat_id).await?;
                bot.send_chat_action(chat_id, ChatAction::Typing).await?;
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
        },
        Command::Unread => {
            let items = storage.get_all().await?;

            if items.is_empty() {
                bot.send_message(chat_id, "You have no entries yet").await?;
                return Ok(());
            }

            for item in items.values() {
                item.send_to_chat(&bot, chat_id).await?;
                bot.send_chat_action(chat_id, ChatAction::Typing).await?;

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }

            bot.send_message(chat_id, tg_escape("That's all!")).await?;
        }
    }

    Ok(())
}

pub async fn add_new_entry<B: StorageBackend + Debug>(
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
