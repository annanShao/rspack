use core::hash::Hash;
use std::cmp::{Eq, Ordering};
use std::rc::Rc;
use std::sync::Arc;

use itertools::Itertools;
use rspack_database::Database;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

use crate::chunk_combination::{ChunkCombination, ChunkCombinationUkey};

// type LeafSortedAlgorithm = dyn Fn(ChunkCombination, ChunkCombination) -> Ordering;
// type BucketSortedAlgorithm = dyn Fn(u64, u64) -> Ordering;
// type BucketGetKeyFunc = dyn Fn(ChunkCombination) -> u64;

enum BucketMapVariant {
  HashSetVariant(HashSet<ChunkCombinationUkey>),
  LazyBucketVariant(LazyBucketSortedSet),
}

impl BucketMapVariant {
  fn insert_item_for_entry(&mut self, item: ChunkCombinationUkey) {
    match self {
      BucketMapVariant::HashSetVariant(ref mut entry_inner_hash_set) => {
        entry_inner_hash_set.insert(item);
      }
      BucketMapVariant::LazyBucketVariant(ref mut entry_inner_bucket_set) => {
        entry_inner_bucket_set.add(item);
      }
    }
  }
}

pub enum FinishUpdateFuncs {
  finish_update_first(Box<dyn Fn() -> ()>),
  finish_update_second(Box<dyn Fn() -> ()>),
}

impl FinishUpdateFuncs {
  fn finish(&self) {
    match self {
      FinishUpdateFuncs::finish_update_first(func) => func(),
      FinishUpdateFuncs::finish_update_second(func) => func(),
    }
  }
}

// LimitChunkCountPlugin's useonly datastruct which could improve the performance about get the best mergedChunk.
pub struct LazyBucketSortedSet {
  // bucket_sorted_algorithms: Vec<Box<BucketSortedAlgorithm>>,
  // leaf_sorted_algorithm: Box<LeafSortedAlgorithm>,
  // bucket_get_key_funcs: Vec<Box<BucketGetKeyFunc>>,
  // is_leaf: bool,
  layer: u32,
  // get_key: Box<BucketGetKeyFunc>,
  // comparator: Box<BucketSortedAlgorithm>,
  keys: HashSet<u64>,
  map: HashMap<u64, BucketMapVariant>,
  unsorted_items: HashSet<ChunkCombinationUkey>, // chunk_combination
  size: u32,
}

impl LazyBucketSortedSet {
  pub fn new(
    // bucket_sorted_algorithms: Vec<Box<BucketSortedAlgorithm>>,
    // leaf_sorted_algorithm: Box<LeafSortedAlgorithm>,
    // bucket_get_key_funcs: Vec<Box<BucketGetKeyFunc>>,
    layer: u32,
  ) -> Self {
    // let is_leaf = if bucket_get_key_funcs.is_empty() {
    //   true
    // } else {
    //   false
    // };
    // TODO 添加 layer
    // let next_funcs = bucket_get_key_funcs[1..].to_vec();
    // let get_key = Box::new(bucket_get_key_funcs[0].as_ref());
    // let comparator = Box::new(bucket_sorted_algorithms[0].as_ref());

    Self {
      // bucket_get_key_funcs,
      // leaf_sorted_algorithm,
      // bucket_sorted_algorithms,
      // is_leaf,
      // get_key,
      // comparator,
      layer,
      keys: Default::default(),
      map: Default::default(),
      unsorted_items: Default::default(),
      size: 0,
    }
  }

  pub fn delete_key(&mut self, key: u64) {
    self.keys.remove(&key);
    self.map.remove(&key);
  }

  pub fn add(&mut self, item: ChunkCombinationUkey) {
    self.size += 1;
    self.unsorted_items.insert(item);
  }

  fn get_key_fn_by_layer(&self) -> impl Fn(ChunkCombination) -> u64 {
    match self.layer {
      0 => |a: ChunkCombination| a.size_diff,
      1 => |a: ChunkCombination| a.integrated_size,
      2 => |a: ChunkCombination| (a.b_idx - a.a_idx) as u64,
      _ => todo!(),
    }
  }

  fn get_bucket_sort_func(&self) -> impl Fn(u64, u64) -> Ordering {
    match self.layer {
      0 => |a: u64, b: u64| {
        if a > b {
          Ordering::Less
        } else if a == b {
          Ordering::Equal
        } else {
          Ordering::Greater
        }
      },
      1 => |a: u64, b: u64| {
        if a > b {
          Ordering::Greater
        } else if a == b {
          Ordering::Equal
        } else {
          Ordering::Less
        }
      },
      2 => |a: u64, b: u64| {
        if a > b {
          Ordering::Greater
        } else if a == b {
          Ordering::Equal
        } else {
          Ordering::Less
        }
      },
      _ => todo!(),
    }
  }

  fn get_leaf_sorted_func(&self) -> impl Fn(usize, usize) -> Ordering {
    |a: usize, b: usize| {
      if a > b {
        Ordering::Greater
      } else if a == b {
        Ordering::Equal
      } else {
        Ordering::Less
      }
    }
  }

  pub fn delete(
    &mut self,
    item: &ChunkCombinationUkey,
    chunk_combination_by_ukey: &Database<ChunkCombination>,
  ) {
    self.size -= 1;
    if self.unsorted_items.contains(item) {
      self.unsorted_items.remove(item);
    } else {
      let chunk_combination = chunk_combination_by_ukey
        .get(item)
        .expect("It won't happen.");
      let key = (self.get_key_fn_by_layer())(chunk_combination.clone());
      let mut entry = self.map.get_mut(&key).expect("It won't happen.");
      match &mut entry {
        BucketMapVariant::HashSetVariant(entry_inner_hash_set) => {
          entry_inner_hash_set.remove(item);
          if entry_inner_hash_set.len() == 0 {
            self.delete_key(key);
          }
        }
        BucketMapVariant::LazyBucketVariant(entry_inner_bucket_set) => {
          entry_inner_bucket_set.delete(item, chunk_combination_by_ukey);
          if entry_inner_bucket_set.size == 0 {
            self.delete_key(key);
          }
        }
      }
    }
  }

  pub fn add_internal(&mut self, key: u64, item: ChunkCombinationUkey) {
    if let Some(entry) = self.map.get_mut(&key) {
      entry.insert_item_for_entry(item);
    } else {
      let mut entry_new: BucketMapVariant = match self.layer {
        3 => BucketMapVariant::HashSetVariant(Default::default()),
        _ => BucketMapVariant::LazyBucketVariant(LazyBucketSortedSet::new(self.layer + 1)),
      };
      self.keys.insert(key);
      entry_new.insert_item_for_entry(item);
      self.map.insert(key, entry_new);
    }
  }

  pub fn pop_first(
    &mut self,
    chunk_combination_by_ukey: &Database<ChunkCombination>,
  ) -> Option<ChunkCombinationUkey> {
    if self.size == 0 {
      return None;
    } else {
      self.size -= 1;
      if !self.unsorted_items.is_empty() {
        self
          .unsorted_items
          .clone()
          .into_iter()
          .for_each(|item: ChunkCombinationUkey| {
            let chunk_combination = chunk_combination_by_ukey
              .get(&item)
              .expect("It won't happen.");
            let key = (self.get_key_fn_by_layer())(chunk_combination.clone());
            self.add_internal(key, item)
          });
        self.unsorted_items.clear();
      }
      let sorted_keys: Vec<u64> = self
        .keys
        .clone()
        .into_iter()
        .sorted_by(|a, b| (self.get_bucket_sort_func())(*a, *b))
        .collect();
      let key = sorted_keys[0];
      let comparator = self.get_leaf_sorted_func();
      if self.layer == 3 {
        let entry = self.map.get(&key).expect("It won't happen.");
        match entry {
          BucketMapVariant::HashSetVariant(leaf_hash_set) => {
            let mut sorted_leaf_hash_set: Vec<ChunkCombinationUkey> = leaf_hash_set
              .clone()
              .into_iter()
              .sorted_by(|a, b| {
                let a_b_idx = chunk_combination_by_ukey
                  .get(a)
                  .expect("It won't happen.")
                  .b_idx;
                let b_b_idx = chunk_combination_by_ukey
                  .get(b)
                  .expect("It won't happen.")
                  .b_idx;
                comparator(a_b_idx, b_b_idx)
              })
              .collect();
            let first_item = sorted_leaf_hash_set.remove(0);
            let new_leaf_hash_set: HashSet<ChunkCombinationUkey> =
              sorted_leaf_hash_set.into_iter().collect();
            if new_leaf_hash_set.is_empty() {
              self.delete_key(key);
            } else {
              self
                .map
                .insert(key, BucketMapVariant::HashSetVariant(new_leaf_hash_set));
            }
            return Some(first_item.clone());
          }
          BucketMapVariant::LazyBucketVariant(_) => unreachable!(),
        }
      } else {
        let entry = self.map.get_mut(&key).expect("It won't happen.");
        match entry {
          BucketMapVariant::LazyBucketVariant(node_bucket_set) => {
            let first_item = node_bucket_set.pop_first(chunk_combination_by_ukey);
            if node_bucket_set.size == 0 {
              self.delete_key(key);
            }
            return first_item;
          }
          BucketMapVariant::HashSetVariant(_) => unreachable!(),
        }
      }
    }
  }

  pub fn finish_first(&mut self, item: ChunkCombinationUkey) {
    self.unsorted_items.remove(&item);
    self.size -= 1;
  }

  pub fn start_update(
    &mut self,
    item: &ChunkCombinationUkey,
    chunk_combination_by_ukey: &Database<ChunkCombination>,
  ) -> FinishUpdateFuncs {
    if self.unsorted_items.contains(item) {
      let item_ref = Rc::new(item.clone());
      let self_ref = Rc::new(std::cell::RefCell::new(self));
      return self.finish_first(item.clone())
    } else {
      let chunk_combination = chunk_combination_by_ukey
        .get(item)
        .expect("It won't happen.");
      let key = (self.get_key_fn_by_layer())(chunk_combination.clone());
      if self.layer == 3 {
        let old_entry = self.map.get(&key).expect("It won't happen.");
        match old_entry {
          BucketMapVariant::HashSetVariant(leaf_hash_set) => {}
          BucketMapVariant::LazyBucketVariant(_) => unreachable!(),
        }
      } else {
        let old_entry = self.map.get_mut(&key).expect("It won't happen.");
        match old_entry {
          BucketMapVariant::LazyBucketVariant(bucket_set) => {
            let finish_update = bucket_set.start_update(item, chunk_combination_by_ukey);
          }
          BucketMapVariant::HashSetVariant(_) => unreachable!(),
        }
      }

      return FinishUpdateFuncs::finish_update_second(Box::new(|| {
        // self.unsorted_items.remove(item);
        // self.size -= 1;
        ()
      }));
    }
  }
}
