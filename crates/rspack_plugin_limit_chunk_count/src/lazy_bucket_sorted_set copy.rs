use core::hash::Hash;
use std::cmp::{Eq, Ordering};
use std::sync::Arc;

use itertools::Itertools;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

type BucketSortedAlgorithm<T> = dyn Fn(&T, &T) -> Ordering;
type BucketGetKeyFunc<K, T> = dyn Fn(&T) -> K;

enum BucketMapVariant<K, T> {
  HashSetVariant(HashSet<T>),
  LazyBucketVariant(LazyBucketSortedSet<K, T>),
}

enum BucketMapSortAlgorithm<K, T> {
  LeafAlgorithm(Box<BucketSortedAlgorithm<T>>),
  NodeAlgorithm(Box<BucketSortedAlgorithm<K>>),
}

struct InnerSet<K, T> {
  inner: BucketMapVariant<K, T>,
  comparator: Option<Arc<Box<BucketSortedAlgorithm<T>>>>,
}

pub struct LazyBucketSortedSet<K, T> {
  bucket_sorted_algorithms: Vec<Arc<BucketMapSortAlgorithm<K, T>>>,
  bucket_get_key_funcs: Vec<Arc<BucketGetKeyFunc<K, T>>>,
  is_leaf: bool,
  get_key: Arc<BucketGetKeyFunc<K, T>>,
  comparator: Arc<BucketMapSortAlgorithm<K, T>>,
  keys: HashSet<K>,
  map: HashMap<K, InnerSet<K, T>>,
  unsorted_items: HashSet<T>, // chunk_combination
  size: u32,
}

impl<K, T> LazyBucketSortedSet<K, T>
where
  K: Hash + Eq + Clone + Copy,
  T: Hash + Eq + PartialEq + Clone,
{
  pub fn new(
    mut bucket_sorted_algorithms: Vec<Arc<BucketMapSortAlgorithm<K, T>>>,
    mut bucket_get_key_funcs: Vec<Arc<BucketGetKeyFunc<K, T>>>,
  ) -> Self {
    let mut is_leaf = false;
    if bucket_get_key_funcs.len() == 0 {
      is_leaf = true;
    }
    let get_key = bucket_get_key_funcs.remove(0).clone();
    let comparator = bucket_sorted_algorithms.remove(0).clone();
    Self {
      get_key,
      comparator,
      bucket_get_key_funcs,
      bucket_sorted_algorithms,
      is_leaf,
      keys: Default::default(),
      map: Default::default(),
      unsorted_items: Default::default(),
      size: 0,
    }
  }

  pub fn delete_key(&mut self, key: &K) {
    self.keys.remove(key);
    self.map.remove(key);
  }

  pub fn add(&mut self, item: T) {
    self.size += 1;
    self.unsorted_items.insert(item);
  }

  pub fn delete(&mut self, item: &T) {
    self.size -= 1;
    if self.unsorted_items.contains(item) {
      self.unsorted_items.remove(item);
    } else {
      let key = self.get_key.as_ref()(item);
      let entry = self.map.get_mut(&key).expect("It will not happen.");
      match &mut entry.inner {
        BucketMapVariant::HashSetVariant(entry_inner_hash_set) => {
          entry_inner_hash_set.remove(item);
          if entry_inner_hash_set.len() == 0 {
            self.delete_key(&key);
          }
        }
        BucketMapVariant::LazyBucketVariant(entry_inner_bucket_set) => {
          entry_inner_bucket_set.delete(item);
          if entry_inner_bucket_set.size == 0 {
            self.delete_key(&key);
          }
        }
      }
    }
  }

  fn insert_item_for_entry(&self, entry: &mut InnerSet<K, T>, item: T) {
    match &mut entry.inner {
      BucketMapVariant::HashSetVariant(ref mut entry_inner_hash_set) => {
        entry_inner_hash_set.insert(item);
      }
      BucketMapVariant::LazyBucketVariant(ref mut entry_inner_bucket_set) => {
        entry_inner_bucket_set.add(item);
      }
    }
  }

  pub fn add_internal(&mut self, key: K, item: T) {
    let entry = self.map.get_mut(&key);
    if entry.is_none() {
      let entry = match self.is_leaf {
        true => {
          let cmp = match self.bucket_sorted_algorithms[0].as_ref() {
            BucketMapSortAlgorithm::LeafAlgorithm(leaf_cmp) => leaf_cmp,
            BucketMapSortAlgorithm::NodeAlgorithm(_) => unreachable!(),
          };
          InnerSet {
            inner: BucketMapVariant::HashSetVariant(Default::default()),
            comparator: Some(Arc::new(*cmp)),
          }
        }
        false => InnerSet {
          inner: BucketMapVariant::LazyBucketVariant(Self::new(
            self.bucket_sorted_algorithms.clone(),
            self.bucket_get_key_funcs.clone(),
          )),
          comparator: None,
        },
      };
      self.keys.insert(key.clone());
      self.map.insert(key.clone(), entry);
    } else {
      let mut entry = entry.expect("It cannot happen.");
      self.insert_item_for_entry(entry, item);
    }
  }

  pub fn pop_first(&mut self) -> Option<T> {
    if self.size == 0 {
      return None;
    }
    self.size -= 1;
    if self.unsorted_items.len() > 0 {
      self.unsorted_items.iter().for_each(|item| {
        let key = self.get_key.as_ref()(item);
        self.add_internal(key, item.clone());
      });
      self.unsorted_items.clear();
    }

    self
      .keys
      .iter()
      .sorted_by(|a, b| match self.comparator.as_ref() {
        BucketMapSortAlgorithm::NodeAlgorithm(cmp) => cmp(a, b),
        _ => unreachable!(),
      });
    let Some(key) = self.keys.iter().next();
    let mut entry = self.map.get(key).expect("It should not happen.");
    if self.is_leaf {
      let mut leaf_entry = entry;
      // should be hash_set
      match entry.inner {
        BucketMapVariant::HashSetVariant(mut hash_set_entry) => {
          hash_set_entry
            .iter()
            .sorted_by(|a, b| leaf_entry.comparator.expect("It should not happen.")(a, b)); // TODO: test
          let Some(current_item) = hash_set_entry.iter().next();
          hash_set_entry.remove(current_item);
          if hash_set_entry.len() == 0 {
            self.delete_key(key);
          }
          return Some(current_item.clone());
        }
        _ => None,
      }
    } else {
      match entry.inner {
        BucketMapVariant::LazyBucketVariant(mut bucket_set_entry) => {
          let Some(current_item) = bucket_set_entry.pop_first();
          if bucket_set_entry.size == 0 {
            self.delete_key(key);
          }
          return Some(current_item.clone());
        }
        _ => None,
      }
    }
  }
}

// TODO test
