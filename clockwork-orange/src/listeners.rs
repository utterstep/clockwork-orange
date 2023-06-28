use axum::routing::get;
use color_eyre::{eyre::WrapErr, Result};
use log::info;
use teloxide::{
    dispatching::update_listeners::{
        polling_default,
        webhooks::{self, Options},
    },
    prelude::LoggingErrorHandler,
    requests::Requester,
    update_listeners::UpdateListener,
};

use crate::{bot::Dispatcher, config::Config};

/// Polling listener
pub async fn start_polling<R>(mut dispatcher: Dispatcher<'_>, bot: R) -> Result<()>
where
    R: Requester + Send + 'static,
    <R as Requester>::GetUpdates: Send,
{
    info!("Starting bot in polling mode");

    let error_handler = LoggingErrorHandler::new();
    let listener = polling_default(bot).await;

    dispatcher
        .dispatch_with_listener(listener, error_handler)
        .await;

    Ok(())
}

/// Webhook (axum) listener
pub async fn start_webhook<R>(mut dispatcher: Dispatcher<'_>, bot: R, config: &Config) -> Result<()>
where
    R: Requester + Send + 'static,
    <R as Requester>::GetUpdates: Send,
    <R as Requester>::DeleteWebhook: Send,
    <R as Requester>::Err: Sync,
{
    info!("Starting bot in webhook mode");
    let error_handler = LoggingErrorHandler::new();
    let options = Options::new(config.bind_to, config.webhook_url.clone());

    let (mut listener, stop_flag, router) = webhooks::axum_to_router(bot, options)
        .await
        .wrap_err("failure while creating router")?;
    let stop_token = listener.stop_token();
    let bind_to = config.bind_to;

    tokio::spawn(async move {
        axum::Server::bind(&bind_to)
            .serve(
                router
                    .route("/health", get(health_check))
                    .into_make_service(),
            )
            .with_graceful_shutdown(stop_flag)
            .await
            .map_err(|err| {
                stop_token.stop();
                err
            })
            .expect("Axum server error");
    });

    dispatcher
        .dispatch_with_listener(listener, error_handler)
        .await;

    Ok(())
}

async fn health_check() -> String {
    "OK".into()
}
