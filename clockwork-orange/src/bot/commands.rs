use teloxide::utils::command::BotCommands;

#[derive(Debug, PartialEq, Eq, Clone, Copy, BotCommands)]
#[command(rename_rule = "snake_case", description = "Available commands")]
/// Available commands
pub enum Command {
    /// /start command
    #[command(description = "Start the bot")]
    Start,
    /// Get all items created by current user
    #[command(description = "Get all items created by current user")]
    AllMy,
    /// Get random item from collection
    #[command(description = "Get random item from collection")]
    Random,
    /// Get all unread items
    #[command(description = "Get all unread items")]
    Unread,
}
