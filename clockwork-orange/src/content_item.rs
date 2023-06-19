use std::borrow::Borrow;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct ContentItem {
    /// Author of an item
    author: String,
    /// URL of an item
    content: String,
    /// Whether the user has "read" the content item and when – in UTC
    read_at: Option<OffsetDateTime>,
}

impl ContentItem {
    /// Constructor method, newly created ContentItem presumes to be unread
    pub fn new(author: impl Borrow<str>, content: impl Borrow<str>) -> Self {
        Self {
            author: author.borrow().to_string(),
            content: content.borrow().to_string(),
            read_at: None,
        }
    }
}

/// Getters and setters for the struct
impl ContentItem {
    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    #[allow(dead_code)]
    pub fn set_content(&mut self, content: impl Borrow<str>) {
        self.content = content.borrow().to_string();
    }

    pub fn is_read(&self) -> bool {
        self.read_at.is_some()
    }

    #[allow(dead_code)]
    pub fn set_read(&mut self, read_at: OffsetDateTime) {
        self.read_at.replace(read_at);
    }

    #[allow(dead_code)]
    pub fn set_unread(&mut self) {
        self.read_at = None;
    }
}

/// Methods for sending content items to chats
impl ContentItem {
    /// Convert the item to a Telegram message text, escaping special characters
    pub fn to_tg_message_text(&self) -> String {
        use clockwork_orange_messages::tg_escape;

        let text = format!("suggested by @{}:\n\n{}", self.author(), self.content());

        tg_escape(&text)
    }
}
