use teloxide::types::{CallbackQuery, MediaKind, MediaText, Message, MessageKind, User};

use super::callbacks::Callback;

pub(super) fn get_message_author(msg: Message) -> Option<User> {
    if let MessageKind::Common(msg) = msg.kind {
        return msg.from;
    }

    None
}

pub(super) fn get_message_text(msg: Message) -> Option<MediaText> {
    if let MessageKind::Common(msg) = msg.kind {
        if let MediaKind::Text(text) = msg.media_kind {
            return Some(text);
        }
    }

    None
}

pub(super) fn get_callback_data(query: CallbackQuery) -> Option<Callback> {
    Callback::from_payload(&query.data?)
}
