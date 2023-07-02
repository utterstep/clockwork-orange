//! Redis storage backend.
//!
//! I've chosen Redis as a storage backend because it's provided free with Fly.io :)
//! It'd be much easier to use something more relational, like Postgres,
//! but I don't want to pay for it currently.
//!
//! TODO: think about [Neon](https://neon.tech) as a solution for Postgres.

use std::{fmt, time::Duration};

use bincode::{deserialize, serialize};
use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use log::{debug, info};
use redis::{aio::Connection, AsyncCommands, Client};
use tokio::time::timeout;

use super::{ContentItem, Key, StorageBackend};

#[derive(Clone)]
pub struct RedisStorage {
    client: Client,
}

impl fmt::Debug for RedisStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisStorage").finish()
    }
}

impl RedisStorage {
    pub async fn new(url: &str) -> Result<Self> {
        let client = Client::open(url)?;

        Ok(Self { client })
    }

    pub fn into_storage(self) -> super::Storage<Self> {
        super::Storage { backend: self }
    }

    /// Method do get live redis connection.
    ///
    /// Currently creates new connection every time.
    /// Huge performance impact, but for current state of having only 2 (two) users
    /// that's probably fine :)
    async fn connection(&self) -> Result<Connection> {
        info!("Getting redis connection");

        let connection = timeout(
            Duration::from_millis(1500),
            self.client.get_tokio_connection(),
        )
        .await
        .wrap_err("connection didn't established before timeout")?
        .wrap_err("Redis connection error")?;

        info!("Got redis connection");

        Ok(connection)
    }
}

#[async_trait::async_trait]
impl StorageBackend for RedisStorage {
    async fn get(&self, key: &Key) -> Result<Option<ContentItem>> {
        let mut connection = self.connection().await?;

        let item: Option<Vec<u8>> = connection
            .get(key.as_ref())
            .await
            .wrap_err_with(|| format!("failed to get item from Redis by key `{key:?}`"))?;

        Ok(item
            .map(|item| deserialize(&item).wrap_err("failed to deserialize item in `get`"))
            .transpose()?)
    }

    async fn set(&mut self, key: &Key, value: ContentItem) -> Result<()> {
        let mut connection = self.connection().await?;

        let item = serialize(&value).wrap_err("failed to serialize item in `set`")?;
        connection
            .set(key.as_ref(), item)
            .await
            .wrap_err("failed to set item via Redis?")?;

        Ok(())
    }

    async fn get_all(&self) -> Result<std::collections::HashMap<Key, ContentItem>> {
        let mut connection = self.connection().await?;

        let keys: Vec<String> = connection
            .keys("*")
            .await
            .wrap_err("failed to get keys from Redis")?;

        let mut items = std::collections::HashMap::new();
        for key in keys {
            let item = connection
                .get::<_, Option<Vec<u8>>>(&key).await
                .wrap_err_with(|| format!("failed to get item from Redis by key `{key}`"))?
                .ok_or_else(|| eyre!("failed to get item from Redis by key that Redis provided. Most likely we've encountered a race here"))?;

            let item: ContentItem =
                deserialize(&item).wrap_err("failed to deserialize item in `get_all`")?;

            if item.is_read() {
                continue;
            }

            items.insert(Key(key), item);
        }
        Ok(items)
    }

    async fn get_user_items(
        &self,
        user: &str,
    ) -> Result<std::collections::HashMap<Key, ContentItem>> {
        // TODO: think about prefixing keys with user name
        let mut map = self.get_all().await?;
        map.retain(|_, item| item.author() == user);

        Ok(map)
    }

    async fn get_now(&self) -> Result<time::OffsetDateTime> {
        let mut connection = self.connection().await?;

        let (seconds, _usecs): (i64, i64) = redis::cmd("TIME")
            .query_async(&mut connection)
            .await
            .wrap_err("failed to get time from Redis")?;

        Ok(time::OffsetDateTime::from_unix_timestamp(seconds)?)
    }

    async fn delete(&mut self, key: &Key) -> Result<()> {
        let mut connection = self.connection().await?;

        connection
            .del(key.as_ref())
            .await
            .wrap_err_with(|| format!("failed to delete item by key {key:?}"))?;
        Ok(())
    }

    async fn get_random(&self) -> Result<Option<(Key, ContentItem)>> {
        let mut connection = self.connection().await?;

        // TODO: this will be slow as hell when we'll have a lot of read items
        loop {
            let key: Option<String> = redis::cmd("RANDOMKEY")
                .query_async(&mut connection)
                .await
                .wrap_err("failed to get random key from Redis")?;

            debug!("got following random key: {key:?}");

            if let Some(key) = key {
                let item: Vec<u8> = connection
                    .get::<_, Option<Vec<u8>>>(&key)
                    .await?
                    .ok_or_else(|| eyre!("key {key:?} got from RANDOMKEY doesn't exist! Most likely we've encountered a race here"))?;
                let item: ContentItem =
                    deserialize(&item).wrap_err("failed to deserialize item in `get_random`")?;

                if item.is_read() {
                    continue;
                }

                return Ok(Some((key.into(), item)));
            } else {
                return Ok(None);
            }
        }
    }

    async fn health_check(&self) -> Result<()> {
        let mut connection = self.connection().await?;

        let _: String = redis::cmd("PING")
            .query_async(&mut connection)
            .await
            .wrap_err("failed to ping Redis")?;

        Ok(())
    }
}
