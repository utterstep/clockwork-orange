use std::fmt;

use bincode::{deserialize, serialize};
use color_eyre::Result;
use redis::{aio::ConnectionManager, AsyncCommands, Client};

use super::{ContentItem, Key, StorageBackend};

#[derive(Clone)]
pub struct RedisStorage {
    conn_manager: ConnectionManager,
}

impl fmt::Debug for RedisStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RedisStorage").finish()
    }
}

impl RedisStorage {
    pub async fn new(url: &str) -> Result<Self> {
        let client = Client::open(url).unwrap();
        let conn_manager = client.get_tokio_connection_manager().await?;
        Ok(Self { conn_manager })
    }

    pub fn into_storage(self) -> super::Storage<Self> {
        super::Storage { backend: self }
    }

    /// Method do get live redis connection.
    /// If connection is not available, it will try to reconnect once.
    /// If it fails again, it will return the error.
    ///
    /// TODO: disallow using `self.conn_manager` directly on typesystem level
    async fn connection(&self) -> Result<ConnectionManager> {
        let mut conn_manager = self.conn_manager.clone();

        match redis::cmd("PING").query_async(&mut conn_manager).await {
            Ok(()) => {}
            Err(_) => {
                // wait once for reconnection
                // if it fails again, return the error
                redis::cmd("PING").query_async(&mut conn_manager).await?;
            }
        }

        Ok(conn_manager)
    }
}

#[async_trait::async_trait]
impl StorageBackend for RedisStorage {
    async fn get(&self, key: &Key) -> Result<Option<ContentItem>> {
        let mut conn_manager = self.connection().await?;

        let item: Option<Vec<u8>> = conn_manager.get(key.as_ref()).await?;
        Ok(item.map(|item| deserialize(&item).unwrap()))
    }

    async fn set(&mut self, key: &Key, value: ContentItem) -> Result<()> {
        let item = serialize(&value).unwrap();
        self.conn_manager.set(key.as_ref(), item).await?;
        Ok(())
    }

    async fn get_all(&self) -> Result<std::collections::HashMap<Key, ContentItem>> {
        let mut conn_manager = self.connection().await?;

        let keys: Vec<String> = conn_manager.keys("*").await?;
        let mut items = std::collections::HashMap::new();
        for key in keys {
            let item: Option<Vec<u8>> = conn_manager.get(&key).await?;
            if let Some(item) = item {
                items.insert(Key(key), deserialize(&item).unwrap());
            }
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
        let mut conn_manager = self.connection().await?;

        let (seconds, _usecs): (i64, i64) =
            redis::cmd("TIME").query_async(&mut conn_manager).await?;

        Ok(time::OffsetDateTime::from_unix_timestamp(seconds)?)
    }

    async fn delete(&mut self, key: &Key) -> Result<()> {
        let mut conn_manager = self.connection().await?;

        conn_manager.del(key.as_ref()).await?;
        Ok(())
    }

    async fn get_random(&self) -> Result<Option<(Key, ContentItem)>> {
        let mut conn_manager = self.connection().await?;

        let key: Option<String> = redis::cmd("RANDOMKEY")
            .query_async(&mut conn_manager)
            .await?;

        if let Some(key) = key {
            let item: Option<Vec<u8>> = conn_manager.get(&key).await?;
            if let Some(item) = item {
                return Ok(Some((key.into(), deserialize(&item)?)));
            }
        }

        Ok(None)
    }
}
