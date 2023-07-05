use teloxide::types::InlineKeyboardButton;
use tracing::debug;

use crate::storage::Key;

/// Callbacks are used to handle user interaction with bot.
///
/// Currently the only callback type is `mark-as-read`, more may be added in the future.
#[derive(Debug, Clone)]
pub enum Callback {
    MarkAsRead(Key),
}

impl Callback {
    /// Get callback kind as &str
    fn kind_as_str(&self) -> &str {
        match self {
            Self::MarkAsRead(_) => "mark-as-read",
        }
    }

    /// Transform callback to payload for sending to TG API
    pub fn to_payload(&self) -> String {
        let res = match self {
            Self::MarkAsRead(key) => format!("{}:{}", self.kind_as_str(), key.as_ref()),
        };
        debug_assert!(
            res.len() <= 64,
            "callback data is too long for Telegram API: {}",
            res
        );

        res
    }

    /// Create callback item with `mark-as-read` kind
    pub fn mark_as_read(key: &Key) -> Self {
        Self::MarkAsRead(key.clone())
    }

    /// Create button for sending to TG API
    pub fn as_button(&self, text: impl Into<String>) -> InlineKeyboardButton {
        let payload = self.to_payload();

        InlineKeyboardButton::callback(text, payload)
    }

    /// Create callback from TG CallbackQuery payload
    pub fn from_payload(payload: &str) -> Option<Self> {
        debug!("got callback with payload: {payload}");
        let (kind, data) = payload.split_once(':')?;

        match kind {
            "mark-as-read" => {
                let key = Key::from(data.to_string());
                Some(Self::MarkAsRead(key))
            }
            _ => None,
        }
    }
}
