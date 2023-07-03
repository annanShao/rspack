use std::cmp::Ordering;

use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

type BucketSortedAlgorithm<K> = Fn(&K, &K) -> Ordering;
type BucketGetKeyFunc<T, K> = Fn(&T) -> K;

enum BucketMapVariant<T, K> {
  HashSetVariant(HashSet<K>),
  LazyBucketVariant(LazyBucketSortedSet<T, K>),
}

pub struct LazyBucketSortedSet<T, K> {
  bucket_sorted_algorithms: Vec<Box<dyn BucketSortedAlgorithm<K>>>,
  bucket_get_key_funcs: Vec<Box<dyn BucketGetKeyFunc<T, K>>>,
  is_leaf: bool,
  get_key: BucketGetKeyFunc<T, K>,
  comparator: BucketSortedAlgorithm<K>,
  keys: HashSet<K>,
  map: HashMap<K, BucketMapVariant>,
  unsorted_items: HashSet<K>,
  size: u32,
}

impl LazyBucketSortedSet<T, K> {
  fn new(
    bucket_sorted_algorithms: Vec<Box<dyn BucketSortedAlgorithm>>,
    bucket_get_key_funcs: Vec<Box<dyn BucketGetKeyFunc<T, K>>>,
  ) -> Self {
    let mut is_leaf = false;
    if bucket_get_key_funcs.len() == 0 {
      is_leaf = true;
    }
    let get_key = bucket_get_key_funcs.remove(0);
    let comparator = bucket_sorted_algorithms.remove(0);
    Self {
      get_key,
      comparator,
      bucket_get_key_funcs,
      bucket_sorted_algorithms,
      is_leaf,
      keys: HashSet::new(),
      map: HashMap::new(),
      unsorted_items: HashSet::new(),
      size: 0,
    }
  }

  fn delete_key(&mut self, key: &K) {
    self.keys.remove(key);
    self.map.remove(key);
  }

  fn add(&mut self, item: K) {
    self.size += 1;
    self.unsorted_items.insert(item);
  }

  fn delete(&mut self, item: &K) {
    self.size -= 1;
    if self.unsorted_items.contains(item) {
      self.unsorted_items.remove(item);
    } else {
      let key = self.get_key(item);
      let entry = self.map.get(key);
      entry.remove(item);
      if entry.size() == 0 {
        self.delete_key(key);
      }
    }
  }

  fn add_internal(&mut self, key: K, item: T) {
    let mut entry = self.map.get(key);
    if entry.is_none() {
      entry = if self.is_leaf {
        HashSet::new()
      } else {
        Self::new(
          self.bucket_sorted_algorithms.clone(),
          self.bucket_get_key_funcs.clone(),
        )
      };
      self.keys.insert(key);
      self.map.insert(key, entry);
    }
    entry.insert(key);
  }

  fn get_key(&self, &key: T) -> K {
    self.get_key(key)
  }

  fn comparator(&self, a: &K, b: &K) -> Ordering {
    self.comparator(a, b)
  }

  //   fn pop_first(&mut self) -> Option<K> {
  //     if self.size() == 0 {
  //         return None;
  //     }
  //     self.size -= 1;

  //   }
}
