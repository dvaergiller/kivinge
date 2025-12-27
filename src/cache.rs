use std::{collections::HashMap, hash::Hash};

use chrono::{DateTime, TimeDelta, Utc};

const DEFAULT_EXPIRY_SECONDS: i64 = 60;

struct CacheValue<Value> {
    pub updated_at: DateTime<Utc>,
    pub value: Value,
}

pub struct Cache<Key: Eq + Hash, Value> {
    cache: HashMap<Key, CacheValue<Value>>,
    expiry_time: TimeDelta,
}

impl<Key: Eq + Hash, Value> Default for Cache<Key, Value> {
    fn default() -> Self {
        Cache {
            cache: Default::default(),
            expiry_time: TimeDelta::seconds(DEFAULT_EXPIRY_SECONDS),
        }
    }
}

impl<Key: Eq + Hash, Value: Clone> Cache<Key, Value> {
    pub fn lookup(&self, key: &Key) -> Option<&Value> {
        match self.cache.get(key) {
            None => None,
            Some(entry) => {
                let age = Utc::now().signed_duration_since(entry.updated_at);
                if age > self.expiry_time {
                    None
                }
                else {
                    Some(&entry.value)
                }
            }
        }
    }

    pub fn insert(&mut self, key: Key, value: Value) {
        self.cache.insert(key, CacheValue { updated_at: Utc::now(), value });
    }

    pub fn cached_get<Getter, Error>(
        &mut self,
        key: Key,
        getter: Getter
    ) -> Result<Value, Error>
    where Getter: Fn() -> Result<Value, Error>
    {
        if let Some(value) = self.lookup(&key) {
            return Ok(value.clone());
        }

        let value = getter()?;
        self.insert(key, value.clone());
        Ok(value)
    }
}
