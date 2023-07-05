use std::fmt::Debug;

use axum::{extract::State, response::Response, routing::get, Router};
use color_eyre::{eyre::WrapErr, Result};
use teloxide::{
    dispatching::update_listeners::{
        polling_default,
        webhooks::{self, Options},
    },
    prelude::LoggingErrorHandler,
    requests::Requester,
    update_listeners::UpdateListener,
};
use tracing::info;

use crate::{
    bot::Dispatcher,
    config::Config,
    storage::{Storage, StorageBackend},
};

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

#[derive(Clone)]
struct AppState<B: StorageBackend + Debug + 'static> {
    storage: Storage<B>,
}

/// Webhook (axum) listener
pub async fn start_webhook<R, B>(
    mut dispatcher: Dispatcher<'_>,
    bot: R,
    storage: Storage<B>,
    config: &Config,
) -> Result<()>
where
    B: StorageBackend + Debug + 'static,
    R: Requester + Send + 'static,
    <R as Requester>::GetUpdates: Send,
    <R as Requester>::DeleteWebhook: Send,
    <R as Requester>::Err: Sync,
{
    info!("Starting bot in webhook mode");
    let state = AppState { storage };

    let error_handler = LoggingErrorHandler::new();
    let options = Options::new(config.bind_to, config.webhook_url.clone());

    let (mut listener, stop_flag, webhook_router) = webhooks::axum_to_router(bot, options)
        .await
        .wrap_err("failure while creating router")?;
    let health_router = Router::new()
        .route("/health", get(health_check))
        .with_state(state);

    let router = Router::new().merge(webhook_router).merge(health_router);

    let stop_token = listener.stop_token();
    let bind_to = config.bind_to;

    tokio::spawn(async move {
        axum::Server::bind(&bind_to)
            .serve(router.into_make_service())
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

/// Health check endpoint, returns 200 OK if storage is healthy
async fn health_check<B: StorageBackend + Debug + 'static>(
    State(state): State<AppState<B>>,
) -> Response<String> {
    match state.storage.health_check().await {
        Ok(()) => Response::builder().status(200).body("OK".into()).unwrap(),
        Err(err) => Response::builder()
            .status(500)
            .body(format!("Error: {:?}", err))
            .unwrap(),
    }
}
