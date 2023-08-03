use rspack_core::Chunk;
use rspack_database::Ukey;

pub type ChunkCombinationUkey = Ukey<ChunkCombination>;

#[derive(Clone)]
pub struct ChunkCombination {
  pub ukey: ChunkCombinationUkey,
  pub deleted: bool,
  pub size_diff: u64,
  pub integrated_size: u64,
  pub chunk_a: Chunk,
  pub chunk_b: Chunk,
  pub a_idx: usize,
  pub b_idx: usize,
  pub a_size: u64,
  pub b_size: u64,
}
