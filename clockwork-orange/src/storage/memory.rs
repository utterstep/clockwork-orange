//! Memory-based storage.
//!
//! This storage is used for testing purposes only.

use std::collections::HashMap;
use std::sync::Arc;

use color_eyre::Result;
use tokio::sync::{Mutex, MutexGuard};

use super::{ContentItem, Key, StorageBackend};

#[derive(Debug, Default, Clone)]
pub struct MemoryStorage {
    items: Arc<Mutex<HashMap<Key, ContentItem>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn items(&self) -> MutexGuard<HashMap<Key, ContentItem>> {
        self.items.lock().await
    }

    pub fn into_storage(self) -> super::Storage<Self> {
        super::Storage { backend: self }
    }
}

#[async_trait::async_trait]
impl StorageBackend for MemoryStorage {
    async fn get(&self, key: &Key) -> Result<Option<ContentItem>> {
        Ok(self.items().await.get(key).cloned())
    }

    async fn set(&mut self, key: &Key, value: ContentItem) -> Result<()> {
        self.items().await.insert(key.clone(), value);
        Ok(())
    }

    async fn get_all(&self) -> Result<HashMap<Key, ContentItem>> {
        Ok(self
            .items()
            .await
            .iter()
            .filter(|(_, item)| !item.is_read())
            .map(|(key, item)| (key.clone(), item.clone()))
            .collect())
    }

    async fn get_now(&self) -> Result<time::OffsetDateTime> {
        Ok(time::OffsetDateTime::now_utc())
    }

    async fn delete(&mut self, key: &Key) -> Result<()> {
        self.items().await.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    /// Test basic set/get functionality
    async fn test_simple() {
        let mut storage = MemoryStorage::new();
        let key = Key("test".to_string());
        let item = ContentItem::new("test".to_string(), "https://example.com");

        storage.set(&key, item.clone()).await.unwrap();

        assert_eq!(storage.get(&key).await.unwrap(), Some(item));
    }

    #[tokio::test]
    /// Test that `get_all` returns only unread items
    async fn test_get_all() {
        let mut storage = MemoryStorage::new();
        let key = Key("test".to_string());
        let item = ContentItem::new("test".to_string(), "https://example.com");

        storage.set(&key, item).await.unwrap();

        assert!(!storage.get_all().await.unwrap().is_empty());

        storage.mark_as_read(&key).await.unwrap();

        assert!(storage.get_all().await.unwrap().is_empty());
    }

    #[tokio::test]
    /// Test that `get_user_items` returns only unread items, created by the specific user
    async fn test_get_user_items() {
        let mut storage = MemoryStorage::new();
        let alice_key = Key("alice_data".to_string());
        let alice_item = ContentItem::new("alice".to_string(), "https://example.com/alice");
        storage.set(&alice_key, alice_item.clone()).await.unwrap();

        let bob_key = Key("bob_data".to_string());
        let bob_item = ContentItem::new("bob".to_string(), "https://example.com/bob");
        storage.set(&bob_key, bob_item.clone()).await.unwrap();

        assert_eq!(
            storage.get_user_items("alice").await.unwrap(),
            vec![(alice_key.clone(), alice_item.clone())]
                .into_iter()
                .collect()
        );

        assert_eq!(
            storage.get_user_items("bob").await.unwrap(),
            vec![(bob_key.clone(), bob_item.clone())]
                .into_iter()
                .collect()
        );
    }
}
