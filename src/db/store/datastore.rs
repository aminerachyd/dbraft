use std::{collections::HashMap, io::Result};

pub trait DataStore<T> {
    fn get(&self, id: &String) -> Option<&T>;

    fn put(&mut self, id: String, item: T) -> Result<()>;
}

pub struct HashMapStore<T> {
    data: HashMap<String, T>,
}

impl<T> HashMapStore<T> {
    pub fn new() -> Self {
        HashMapStore {
            data: HashMap::new(),
        }
    }
}

impl<T> DataStore<T> for HashMapStore<T> {
    fn get(&self, id: &String) -> Option<&T> {
        self.data.get(id)
    }

    // Always returns an Ok
    fn put(&mut self, id: String, item: T) -> Result<()> {
        self.data.insert(id, item);
        Ok(())
    }
}
