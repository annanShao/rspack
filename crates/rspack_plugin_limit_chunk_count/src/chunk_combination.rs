use rspack_core::Chunk;

pub struct ChunkCombination {
  deleted: bool,
  size_diff: f64,
  integrated_size: f64,
  chunk_a: Chunk,
  chunk_b: Chunk,
  a_idx: usize,
  b_idx: usize,
  a_size: f64,
  b_size: f64,
}
