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
}

#[async_trait::async_trait]
impl StorageBackend for RedisStorage {
    async fn get(&self, key: &Key) -> Result<Option<ContentItem>> {
        let mut conn_manager = self.conn_manager.clone();

        let item: Option<Vec<u8>> = conn_manager.get(key.as_ref()).await?;
        Ok(item.map(|item| deserialize(&item).unwrap()))
    }

    async fn set(&mut self, key: &Key, value: ContentItem) -> Result<()> {
        let item = serialize(&value).unwrap();
        self.conn_manager.set(key.as_ref(), item).await?;
        Ok(())
    }

    async fn get_all(&self) -> Result<std::collections::HashMap<Key, ContentItem>> {
        let mut conn_manager = self.conn_manager.clone();

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
        let conn_manager = self.conn_manager.clone();

        let (seconds, _usecs): (i64, i64) = redis::cmd("TIME")
            .query_async(&mut conn_manager.clone())
            .await?;

        Ok(time::OffsetDateTime::from_unix_timestamp(seconds)?)
    }

    async fn delete(&mut self, key: &Key) -> Result<()> {
        self.conn_manager.del(key.as_ref()).await?;
        Ok(())
    }

    async fn get_random(&self) -> Result<Option<ContentItem>> {
        let mut conn_manager = self.conn_manager.clone();

        let key: Option<String> = redis::cmd("RANDOMKEY")
            .query_async(&mut conn_manager)
            .await?;

        if let Some(key) = key {
            let item: Option<Vec<u8>> = conn_manager.get(&key).await?;
            if let Some(item) = item {
                return Ok(Some(deserialize(&item).unwrap()));
            }
        }

        Ok(None)
    }
}
