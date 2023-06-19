use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use color_eyre::{eyre::eyre, Result};
use rand::Rng;
use time::OffsetDateTime;

use crate::content_item::ContentItem;

mod memory;
pub use memory::MemoryStorage;

mod redis;
pub use self::redis::RedisStorage;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Key(String);

impl Into<Key> for String {
    fn into(self) -> Key {
        Key(self)
    }
}

impl AsRef<str> for Key {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Storage<B: StorageBackend> {
    backend: B,
}

impl<B: StorageBackend> Deref for Storage<B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

impl<B: StorageBackend> DerefMut for Storage<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backend
    }
}

#[async_trait::async_trait]
/// Basic storage trait, all methods, except `get` should return only unread items
pub trait StorageBackend: Send + Sync + Clone + std::fmt::Debug {
    async fn set(&mut self, key: &Key, value: ContentItem) -> Result<()>;
    async fn get(&self, key: &Key) -> Result<Option<ContentItem>>;

    async fn get_all(&self) -> Result<HashMap<Key, ContentItem>>;

    async fn get_user_items(&self, user: &str) -> Result<HashMap<Key, ContentItem>> {
        let mut map = self.get_all().await?;
        map.retain(|_, item| item.author() == user);

        Ok(map)
    }

    async fn get_now(&self) -> Result<OffsetDateTime>;

    async fn delete(&mut self, key: &Key) -> Result<()>;

    async fn mark_as_read(&mut self, key: &Key) -> Result<()> {
        let mut item = self.get(&key).await?;

        if let Some(item) = &mut item {
            item.set_read(self.get_now().await?);
            self.set(&key, item.clone()).await?;
        } else {
            return Err(eyre!("Item not found"));
        }

        Ok(())
    }

    async fn get_random(&self) -> Result<Option<ContentItem>> {
        let items = self.get_all().await?;
        let mut rng = rand::thread_rng();
        if items.is_empty() {
            return Ok(None);
        }
        let index = rng.gen_range(0..items.len());

        let item = items
            .values()
            .nth(index)
            .expect("this should never happen, but it did");
        Ok(Some(item.clone()))
    }
}
