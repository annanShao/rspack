use std::cmp::Ordering;

// Port of https://github.com/webpack/webpack/blob/4b4ca3bb53f36a5b8fc6bc1bd976ed7af161bd80/lib/optimize/LimitChunkCountPlugin.js
// use rspack_core::{Compilation, Plugin};
use itertools::Itertools;
use lazy_bucket_sorted_set::LazyBucketSortedSet;
use rspack_core::{
  chunk_graph_chunk::ChunkSizeOptions, Chunk, Compilation, LimitChunkCountConfig, Plugin,
};
mod chunk_combination;
use chunk_combination::{ChunkCombination, ChunkCombinationUkey};
use rspack_database::Database;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
mod lazy_bucket_sorted_set;

#[derive(Debug)]
pub struct LimitChunkCountPlugin {
  options: LimitChunkCountConfig,
  chunk_combination_by_ukey: Database<ChunkCombination>,
}

impl LimitChunkCountPlugin {
  pub fn new(options: LimitChunkCountConfig) -> Self {
    if options.max_chunks < 1 {
      panic!("Limit the maximum number of chunks must use a value greater than or equal to 1");
    }
    Self {
      options,
      chunk_combination_by_ukey: Default::default(),
    }
  }

  pub fn limit_chunk_count(&self, compilation: &mut Compilation) {
    let chunk_graph = &mut compilation.chunk_graph;
    let max_chunks = self.options.max_chunks;
    let raw_chunks = compilation.chunk_by_ukey.iter().count() as u32;
    if raw_chunks <= max_chunks {
      return ();
    }

    let remaining_chunks_to_merge = raw_chunks - max_chunks;
    let chunks = compilation.chunk_by_ukey.values_mut();
    let ordered_chunks: Vec<&mut Chunk> = chunks
      .into_iter()
      .sorted_by(|a, b| chunk_graph.compare_chunks(a, b))
      .collect();

    // let bucket_sorted_algorithms: Vec<Box<dyn Fn(u64, u64) -> Ordering>> = vec![
    //   Box::new(|a: u64, b: u64| {
    //     if a > b {
    //       Ordering::Less
    //     } else if a == b {
    //       Ordering::Equal
    //     } else {
    //       Ordering::Greater
    //     }
    //   }),
    //   Box::new(|a: u64, b: u64| {
    //     if a > b {
    //       Ordering::Greater
    //     } else if a == b {
    //       Ordering::Equal
    //     } else {
    //       Ordering::Less
    //     }
    //   }),
    // ];

    // fn get_key_of_size_diff(a: ChunkCombination) -> u64 {
    //   a.size_diff
    // }

    // fn get_key_of_integrated_size(a: ChunkCombination) -> u64 {
    //   a.integrated_size
    // }

    // fn get_key_of_idx_diff(a: ChunkCombination) ->u64 {
    //   (a.b_idx - a.a_idx) as u64
    // }

    // let bucket_get_key_funcs: Vec<Box<dyn Fn(ChunkCombination) -> u64>> = vec![
    //   Box::new(|a: ChunkCombination| a.size_diff),
    //   Box::new(|a: ChunkCombination| a.integrated_size),
    //   Box::new(|a: ChunkCombination| (a.b_idx - a.a_idx) as u64),
    // ];

    // fn leaf_sorted_algorithm(a: ChunkCombination, b: ChunkCombination) -> Ordering {
    //   if a.b_idx > b.b_idx {
    //     Ordering::Greater
    //   } else if a.b_idx == b.b_idx {
    //     Ordering::Equal
    //   } else {
    //     Ordering::Less
    //   }
    // };

    // let leaf_sorted_algorithm: Box<dyn Fn(ChunkCombination, ChunkCombination) -> Ordering> =
    //   Box::new(|a: ChunkCombination, b: ChunkCombination| {
    //     if a.b_idx > b.b_idx {
    //       Ordering::Greater
    //     } else if a.b_idx == b.b_idx {
    //       Ordering::Equal
    //     } else {
    //       Ordering::Less
    //     }
    //   });

    let combinations = LazyBucketSortedSet::new(
      // bucket_sorted_algorithms,
      // leaf_sorted_algorithm,
      // bucket_get_key_funcs,
      0,
    );

    let mut combinations_by_chunks: HashMap<Chunk, ChunkCombination> = HashMap::default();
    let options = ChunkSizeOptions {
      entry_chunk_multiplicator: self.options.entry_chunk_multiplicator,
      chunk_overhead: self.options.chunk_overhead,
    };
    for index_b in 0..ordered_chunks.len() {
      let chunk_b = ordered_chunks[index_b].clone();
      for index_a in 0..index_b {
        let chunk_a = ordered_chunks[index_a].clone();
        if !chunk_graph.can_chunks_be_integrated(
          &compilation.chunk_group_by_ukey,
          &chunk_a,
          &chunk_b,
        ) {
          continue;
        }

        let integrated_size = chunk_graph.get_integrated_chunk_size(
          &chunk_a,
          &chunk_b,
          &compilation.module_graph,
          options.clone(),
        );

        let chunk_a_size = chunk_graph.get_chunk_size(
          &chunk_a,
          options.clone(),
          &compilation.module_graph,
          &compilation.chunk_group_by_ukey,
        );
        let chunk_b_size = chunk_graph.get_chunk_size(
          &chunk_b,
          options.clone(),
          &compilation.module_graph,
          &compilation.chunk_group_by_ukey,
        );
        let chunk_combination_ukey = ChunkCombinationUkey::new();

        // TODO 存入到map中
        let comb = ChunkCombination {
          ukey: chunk_combination_ukey,
          deleted: false,
          size_diff: chunk_a_size + chunk_b_size - integrated_size,
          integrated_size,
          chunk_a: chunk_a.clone(),
          chunk_b: chunk_b.clone(),
          a_idx: index_a,
          b_idx: index_b,
          a_size: chunk_a_size,
          b_size: chunk_b_size,
        };
      }
    }
  }
}

#[async_trait::async_trait]
impl Plugin for LimitChunkCountPlugin {
  async fn optimize_chunks(
    &self,
    _ctx: rspack_core::PluginContext,
    args: rspack_core::OptimizeChunksArgs<'_>,
  ) -> rspack_core::PluginOptimizeChunksOutput {
    let compilation = args.compilation;
    self.limit_chunk_count(compilation);
    Ok(())
  }
}
