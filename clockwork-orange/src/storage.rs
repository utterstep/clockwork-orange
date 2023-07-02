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

impl From<String> for Key {
    fn from(val: String) -> Key {
        Key(val)
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
    /// Set item to storage
    async fn set(&mut self, key: &Key, value: ContentItem) -> Result<()>;

    /// Get specific item from storage
    ///
    /// # Caution
    /// May return read items
    async fn get(&self, key: &Key) -> Result<Option<ContentItem>>;

    /// Get all items from storage, filtering unread ones
    async fn get_all(&self) -> Result<HashMap<Key, ContentItem>>;

    /// Get items which this user has added
    async fn get_user_items(&self, user: &str) -> Result<HashMap<Key, ContentItem>> {
        let mut map = self.get_all().await?;
        map.retain(|_, item| item.author() == user);

        Ok(map)
    }

    /// Get current time – we try to connect to external state
    /// through Storage trait whenever possible
    async fn get_now(&self) -> Result<OffsetDateTime>;

    /// Delete item from storage – unused currently
    async fn delete(&mut self, key: &Key) -> Result<()>;

    /// Mark item as "read" – user has seen it and wants to remove it from his queue
    async fn mark_as_read(&mut self, key: &Key) -> Result<()> {
        let mut item = self
            .get(key)
            .await?
            .ok_or_else(|| eyre!("Item not found"))?;

        item.set_read(self.get_now().await?);
        self.set(key, item.clone()).await?;

        Ok(())
    }

    /// Get random unread item from storage
    async fn get_random(&self) -> Result<Option<(Key, ContentItem)>> {
        let items = self.get_all().await?;
        let mut rng = rand::thread_rng();
        if items.is_empty() {
            return Ok(None);
        }
        let index = rng.gen_range(0..items.len());

        let (key, item) = items
            .iter()
            .nth(index)
            .expect("this should never happen, but it did");
        Ok(Some((key.clone(), item.clone())))
    }

    /// Check that storage is live and can be used
    async fn health_check(&self) -> Result<()>;
}
