// Port of https://github.com/webpack/webpack/blob/4b4ca3bb53f36a5b8fc6bc1bd976ed7af161bd80/lib/optimize/LimitChunkCountPlugin.js

// use rspack_core::{Compilation, Plugin};
use rspack_core::{Compilation, LimitChunkCountConfig, Plugin};

use crate::ChunkCombination;

#[derive(Debug)]
pub struct LimitChunkCountPlugin {
  options: LimitChunkCountConfig,
}

impl LimitChunkCountPlugin {
  pub fn new(options: LimitChunkCountConfig) -> Self {
    if options.max_chunks < 1 {
      panic!("Limit the maximum number of chunks must use a value greater than or equal to 1");
    }
    Self { options }
  }

  pub fn limit_chunk_count(&self, compilation: &mut Compilation) {
    let chunk_graph = &mut compilation.chunk_graph;
    let max_chunks = self.options.max_chunks;
    let raw_chunks = compilation.chunk_by_ukey.size() as u32;
    if raw_chunks <= max_chunks {
      return ();
    }

    let remaining_chunks_to_merge = raw_chunks - max_chunks;
    let mut chunks = compilation.chunk_by_ukey.values_mut();
    let ordered_chunks = chunks.into_iter().sorted_by(chunk_graph.compare_chunks);
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
