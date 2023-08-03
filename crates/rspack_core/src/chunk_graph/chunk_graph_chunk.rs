//!  There are methods whose verb is `ChunkGraphChunk`

use std::cmp::Ordering;

use itertools::Itertools;
use rspack_database::Ukey;
use rspack_identifier::{IdentifierLinkedMap, IdentifierSet};
use rspack_util::comparators::{compare_modules_by_identifier, compare_modules_by_identifier_iter};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

use crate::{
  find_graph_roots, BoxModule, Chunk, ChunkByUkey, ChunkGroup, ChunkGroupByUkey, ChunkGroupUkey,
  ChunkUkey, Module, ModuleGraph, ModuleGraphModule, ModuleIdentifier, RuntimeGlobals, SourceType,
};
use crate::{merge_runtime, ChunkGraph};

#[derive(Clone)]
pub struct ChunkSizeOptions {
  pub entry_chunk_multiplicator: Option<i32>,
  pub chunk_overhead: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct ChunkGraphChunk {
  /// URI of modules => ChunkGroupUkey
  ///
  /// use `LinkedHashMap` to keep the ordered from entry array.
  pub(crate) entry_modules: IdentifierLinkedMap<ChunkGroupUkey>,
  pub modules: IdentifierSet,
  pub(crate) runtime_requirements: RuntimeGlobals,
  pub(crate) runtime_modules: Vec<ModuleIdentifier>,
}

impl ChunkGraphChunk {
  pub fn new() -> Self {
    Self {
      entry_modules: Default::default(),
      modules: Default::default(),
      runtime_requirements: Default::default(),
      runtime_modules: Default::default(),
    }
  }
}

impl ChunkGraph {
  pub fn add_chunk(&mut self, chunk_ukey: ChunkUkey) {
    self
      .chunk_graph_chunk_by_chunk_ukey
      .entry(chunk_ukey)
      .or_insert_with(ChunkGraphChunk::new);
  }
  pub fn add_chunk_wit_chunk_graph_chunk(&mut self, chunk_ukey: ChunkUkey, cgc: ChunkGraphChunk) {
    debug_assert!(!self
      .chunk_graph_chunk_by_chunk_ukey
      .contains_key(&chunk_ukey));
    self.chunk_graph_chunk_by_chunk_ukey.insert(chunk_ukey, cgc);
  }

  pub fn get_chunk_entry_modules(&self, chunk_ukey: &ChunkUkey) -> Vec<ModuleIdentifier> {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk_ukey);

    chunk_graph_chunk.entry_modules.keys().cloned().collect()
  }

  pub fn get_chunk_entry_modules_with_chunk_group_iterable(
    &self,
    chunk_ukey: &ChunkUkey,
  ) -> &IdentifierLinkedMap<ChunkGroupUkey> {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    &cgc.entry_modules
  }

  pub(crate) fn get_chunk_graph_chunk_mut(
    &mut self,
    chunk_ukey: ChunkUkey,
  ) -> &mut ChunkGraphChunk {
    self
      .chunk_graph_chunk_by_chunk_ukey
      .get_mut(&chunk_ukey)
      .expect("Chunk should be added before")
  }

  pub(crate) fn get_chunk_graph_chunk(&self, chunk_ukey: &ChunkUkey) -> &ChunkGraphChunk {
    self
      .chunk_graph_chunk_by_chunk_ukey
      .get(chunk_ukey)
      .expect("Chunk should be added before")
  }

  pub(crate) fn disconnect_chunk_and_entry_module(
    &mut self,
    chunk: ChunkUkey,
    module_identifier: ModuleIdentifier,
  ) {
    let chunk_graph_module = self.get_chunk_graph_module_mut(module_identifier);
    chunk_graph_module.entry_in_chunks.remove(&chunk);

    let chunk_graph_chunk = self.get_chunk_graph_chunk_mut(chunk);
    chunk_graph_chunk.entry_modules.remove(&module_identifier);
  }

  pub(crate) fn connect_chunk_and_entry_module(
    &mut self,
    chunk: ChunkUkey,
    module_identifier: ModuleIdentifier,
    entrypoint: ChunkGroupUkey,
  ) {
    let chunk_graph_module = self.get_chunk_graph_module_mut(module_identifier);
    chunk_graph_module.entry_in_chunks.insert(chunk);

    let chunk_graph_chunk = self.get_chunk_graph_chunk_mut(chunk);
    chunk_graph_chunk
      .entry_modules
      .insert(module_identifier, entrypoint);
  }

  pub fn disconnect_chunk_and_module(
    &mut self,
    chunk: &ChunkUkey,
    module_identifier: ModuleIdentifier,
  ) {
    let chunk_graph_module = self.get_chunk_graph_module_mut(module_identifier);
    chunk_graph_module.chunks.remove(chunk);

    let chunk_graph_chunk = self.get_chunk_graph_chunk_mut(*chunk);
    chunk_graph_chunk.modules.remove(&module_identifier);
  }

  pub fn connect_chunk_and_module(
    &mut self,
    chunk: ChunkUkey,
    module_identifier: ModuleIdentifier,
  ) {
    let chunk_graph_module = self.get_chunk_graph_module_mut(module_identifier);
    chunk_graph_module.chunks.insert(chunk);

    let chunk_graph_chunk = self.get_chunk_graph_chunk_mut(chunk);
    chunk_graph_chunk.modules.insert(module_identifier);
  }

  pub fn connect_chunk_and_runtime_module(
    &mut self,
    chunk: ChunkUkey,
    identifier: ModuleIdentifier,
  ) {
    let cgm = self.get_chunk_graph_module_mut(identifier);
    cgm.runtime_in_chunks.insert(chunk);

    let cgc = self.get_chunk_graph_chunk_mut(chunk);
    if !cgc.runtime_modules.contains(&identifier) {
      cgc.runtime_modules.push(identifier);
    }
  }

  pub fn get_chunk_modules<'module>(
    &self,
    chunk: &ChunkUkey,
    module_graph: &'module ModuleGraph,
  ) -> Vec<&'module BoxModule> {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk);
    chunk_graph_chunk
      .modules
      .iter()
      .filter_map(|uri| module_graph.module_by_identifier(uri))
      .collect()
  }

  pub fn get_chunk_module_identifiers(&self, chunk: &ChunkUkey) -> &IdentifierSet {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk);
    &chunk_graph_chunk.modules
  }

  pub fn get_ordered_chunk_modules<'module>(
    &self,
    chunk: &ChunkUkey,
    module_graph: &'module ModuleGraph,
  ) -> Vec<&'module BoxModule> {
    let mut modules = self.get_chunk_modules(chunk, module_graph);
    // SAFETY: module identifier is unique
    modules.sort_unstable_by_key(|m| m.identifier().as_str());
    modules
  }

  pub fn get_chunk_modules_by_source_type<'module>(
    &self,
    chunk: &ChunkUkey,
    source_type: SourceType,
    module_graph: &'module ModuleGraph,
  ) -> Vec<&'module ModuleGraphModule> {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk);
    let modules = chunk_graph_chunk
      .modules
      .iter()
      .filter_map(|uri| module_graph.module_graph_module_by_identifier(uri))
      .filter(|mgm| {
        module_graph
          .module_by_identifier(&mgm.module_identifier)
          .map(|module| module.source_types().contains(&source_type))
          .unwrap_or_default()
      })
      .collect::<Vec<_>>();
    modules
  }

  pub fn get_chunk_modules_iterable_by_source_type<'module_graph: 'me, 'me>(
    &'me self,
    chunk: &ChunkUkey,
    source_type: SourceType,
    module_graph: &'module_graph ModuleGraph,
  ) -> impl Iterator<Item = &'module_graph dyn Module> + 'me {
    let chunk_graph_chunk = self.get_chunk_graph_chunk(chunk);
    chunk_graph_chunk
      .modules
      .iter()
      .filter_map(|uri| module_graph.module_by_identifier(uri))
      .filter(move |module| module.source_types().contains(&source_type))
      .map(|m| m.as_ref())
  }

  pub fn get_chunk_modules_size(&self, chunk: &ChunkUkey, module_graph: &ModuleGraph) -> f64 {
    self
      .get_chunk_modules(chunk, module_graph)
      .iter()
      .fold(0.0, |acc, m| {
        acc + m.source_types().iter().fold(0.0, |acc, t| acc + m.size(t))
      })
  }

  pub fn get_number_of_chunk_modules(&self, chunk: &ChunkUkey) -> usize {
    let cgc = self.get_chunk_graph_chunk(chunk);
    cgc.modules.len()
  }

  pub fn get_number_of_entry_modules(&self, chunk: &ChunkUkey) -> usize {
    let cgc = self.get_chunk_graph_chunk(chunk);
    cgc.entry_modules.len()
  }

  pub fn add_chunk_runtime_requirements(
    &mut self,
    chunk_ukey: &ChunkUkey,
    runtime_requirements: RuntimeGlobals,
  ) {
    let cgc = self.get_chunk_graph_chunk_mut(*chunk_ukey);
    cgc.runtime_requirements.add(runtime_requirements);
  }

  pub fn add_tree_runtime_requirements(
    &mut self,
    chunk_ukey: &ChunkUkey,
    runtime_requirements: RuntimeGlobals,
  ) {
    self.add_chunk_runtime_requirements(chunk_ukey, runtime_requirements);
  }

  pub fn get_chunk_runtime_requirements(&self, chunk_ukey: &ChunkUkey) -> &RuntimeGlobals {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    &cgc.runtime_requirements
  }

  pub fn get_tree_runtime_requirements(&self, chunk_ukey: &ChunkUkey) -> &RuntimeGlobals {
    self.get_chunk_runtime_requirements(chunk_ukey)
  }

  pub fn get_chunk_runtime_modules_in_order(
    &self,
    chunk_ukey: &ChunkUkey,
  ) -> &Vec<ModuleIdentifier> {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    &cgc.runtime_modules
  }

  pub fn get_chunk_runtime_modules_iterable(
    &self,
    chunk_ukey: &ChunkUkey,
  ) -> impl Iterator<Item = &ModuleIdentifier> {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    cgc.runtime_modules.iter()
  }

  pub fn get_chunk_condition_map<F: Fn(&ChunkUkey, &ChunkGraph, &ModuleGraph) -> bool>(
    &self,
    chunk_ukey: &ChunkUkey,
    chunk_by_ukey: &ChunkByUkey,
    chunk_group_by_ukey: &ChunkGroupByUkey,
    module_graph: &ModuleGraph,
    filter: F,
  ) -> HashMap<String, bool> {
    let mut map = HashMap::default();

    let chunk = chunk_by_ukey.get(chunk_ukey).expect("Chunk should exist");
    for c in chunk.get_all_referenced_chunks(chunk_group_by_ukey).iter() {
      let chunk = chunk_by_ukey.get(c).expect("Chunk should exist");
      map.insert(chunk.expect_id().to_string(), filter(c, self, module_graph));
    }

    map
  }

  pub fn get_chunk_root_modules(
    &self,
    chunk: &ChunkUkey,
    module_graph: &ModuleGraph,
  ) -> Vec<ModuleIdentifier> {
    let cgc = self.get_chunk_graph_chunk(chunk);
    let mut input = cgc.modules.iter().cloned().collect::<Vec<_>>();
    input.sort_unstable();
    let mut modules = find_graph_roots(input, |module| {
      let mut set: IdentifierSet = Default::default();
      fn add_dependencies(
        module: ModuleIdentifier,
        set: &mut IdentifierSet,
        module_graph: &ModuleGraph,
      ) {
        let module = module_graph
          .module_by_identifier(&module)
          .expect("should exist");
        for connection in module_graph.get_outgoing_connections(module) {
          // TODO: consider activeState
          // if (activeState === ModuleGraphConnection.TRANSITIVE_ONLY) {
          //   add_dependencies(connection.module_identifier, set, module_graph);
          //   continue;
          // }
          set.insert(connection.module_identifier);
        }
      }

      add_dependencies(module, &mut set, module_graph);
      set.into_iter().collect()
    });

    modules.sort_unstable();

    modules
  }

  pub fn disconnect_chunk(
    &mut self,
    chunk: &mut Chunk,
    chunk_group_by_ukey: &mut ChunkGroupByUkey,
  ) {
    let chunk_ukey = &chunk.ukey;
    let cgc = self.get_chunk_graph_chunk_mut(*chunk_ukey);
    let cgc_modules = std::mem::take(&mut cgc.modules);
    for module in cgc_modules {
      let cgm = self.get_chunk_graph_module_mut(module);
      cgm.chunks.remove(chunk_ukey);
    }
    chunk.disconnect_from_groups(chunk_group_by_ukey)
  }

  pub fn has_chunk_entry_dependent_chunks(
    &self,
    chunk_ukey: &ChunkUkey,
    chunk_group_by_ukey: &ChunkGroupByUkey,
  ) -> bool {
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    for (_, chunk_group_ukey) in cgc.entry_modules.iter() {
      let chunk_group = chunk_group_by_ukey
        .get(chunk_group_ukey)
        .expect("should have chunk group");
      for c in chunk_group.chunks.iter() {
        if c != chunk_ukey {
          return true;
        }
      }
    }
    false
  }

  pub fn get_chunk_entry_dependent_chunks_iterable(
    &self,
    chunk_ukey: &ChunkUkey,
    chunk_by_ukey: &ChunkByUkey,
    chunk_group_by_ukey: &ChunkGroupByUkey,
  ) -> impl Iterator<Item = ChunkUkey> {
    let chunk = chunk_by_ukey.get(chunk_ukey).expect("should have chunk");
    let mut set = HashSet::default();
    for chunk_group_ukey in chunk.groups.iter() {
      let chunk_group = chunk_group_by_ukey
        .get(chunk_group_ukey)
        .expect("should have chunk group");
      if chunk_group.is_initial() {
        let entry_point_chunk = chunk_group.get_entry_point_chunk();
        let cgc = self.get_chunk_graph_chunk(&entry_point_chunk);
        for (_, chunk_group_ukey) in cgc.entry_modules.iter() {
          let chunk_group = chunk_group_by_ukey
            .get(chunk_group_ukey)
            .expect("should have chunk group");
          for c in chunk_group.chunks.iter() {
            let chunk = chunk_by_ukey.get(c).expect("should have chunk");
            if c != chunk_ukey && c != &entry_point_chunk && !chunk.has_runtime(chunk_group_by_ukey)
            {
              set.insert(*c);
            }
          }
        }
      }
    }
    set.into_iter()
  }

  pub fn get_modules_size(
    &self,
    modules: Vec<ModuleIdentifier>,
    module_graph: &ModuleGraph,
  ) -> u64 {
    let mut size: u64 = 0;
    modules
      .iter()
      .filter_map(|module_identifier| module_graph.module_by_identifier(module_identifier))
      .for_each(|module| {
        module.source_types().iter().for_each(|source_type| {
          size = size + module.size(source_type) as u64;
        })
      });
    size
  }

  pub fn get_chunk_size(
    &self,
    chunk: &Chunk,
    options: ChunkSizeOptions,
    module_graph: &ModuleGraph,
    chunk_group_by_ukey: &ChunkGroupByUkey,
  ) -> u64 {
    let chunk_ukey = &chunk.ukey;
    let cgc = self.get_chunk_graph_chunk(chunk_ukey);
    let modules_size =
      self.get_modules_size(cgc.modules.clone().into_iter().collect_vec(), &module_graph);

    let chunk_overhead = if options.chunk_overhead.is_none() {
      10000
    } else {
      options.chunk_overhead.expect("It won't happen.")
    };

    let entry_chunk_multiplicator = if options.entry_chunk_multiplicator.is_none() {
      10
    } else {
      options.entry_chunk_multiplicator.expect("It won't happen.")
    };

    chunk_overhead as u64
      + modules_size
      + if chunk.can_be_initial(chunk_group_by_ukey) {
        entry_chunk_multiplicator as u64
      } else {
        1
      }
  }

  pub fn get_integrated_chunk_size(
    &mut self,
    chunk_a: &Chunk,
    chunk_b: &Chunk,
    module_graph: &ModuleGraph,
    options: ChunkSizeOptions,
  ) -> u64 {
    let chunka_ukey = &chunk_a.ukey;
    let chunkb_ukey = &chunk_b.ukey;
    let cgca = self.get_chunk_graph_chunk_mut(*chunka_ukey);

    let mut all_modules = cgca.modules.clone();

    let cgcb = self.get_chunk_graph_chunk_mut(*chunkb_ukey);

    cgcb.modules.iter().for_each(|module| {
      all_modules.insert(*module);
    });

    let modules_size = self.get_modules_size(all_modules.into_iter().collect_vec(), module_graph);
    let chunk_overhead = if options.chunk_overhead.is_none() {
      10000
    } else {
      options.chunk_overhead.expect("It won't happen.")
    };

    let entry_chunk_multiplicator = if options.entry_chunk_multiplicator.is_none() {
      10
    } else {
      options.entry_chunk_multiplicator.expect("It won't happen.")
    };

    chunk_overhead as u64 + entry_chunk_multiplicator as u64 + modules_size as u64
  }

  pub fn is_available_chunk(
    &self,
    chunk_group_by_ukey: &ChunkGroupByUkey,
    chunk_a: &Chunk,
    chunk_b: &Chunk,
  ) -> bool {
    let mut groups = chunk_b
      .groups
      .iter()
      .cloned()
      .collect::<Vec<Ukey<ChunkGroup>>>();
    while !groups.is_empty() {
      let current_ukey_group = groups.remove(0);
      if chunk_a.is_in_group(&current_ukey_group) {
        continue;
      }
      let chunk_group = chunk_group_by_ukey
        .get(&current_ukey_group)
        .expect("should have chunk group");
      if chunk_group.is_initial() {
        return false;
      }
      chunk_group
        .parents
        .iter()
        .for_each(|chunk_ukey_group_from_parent| {
          groups.push(chunk_ukey_group_from_parent.clone());
        });
    }
    true
  }

  pub fn compare_chunks(&mut self, chunk_a: &Chunk, chunk_b: &Chunk) -> Ordering {
    let chunka_ukey = &chunk_a.ukey;
    let chunkb_ukey = &chunk_b.ukey;
    let cgca = self.get_chunk_graph_chunk(chunka_ukey);
    let cgcb = self.get_chunk_graph_chunk(chunkb_ukey);

    if cgca.modules.len() > cgcb.modules.len() {
      return Ordering::Greater;
    }
    if cgcb.modules.len() > cgca.modules.len() {
      return Ordering::Less;
    }

    let cgca = self.get_chunk_graph_chunk_mut(*chunka_ukey);
    let mut sorted_cgca_modules_iter = cgca
      .modules
      .clone()
      .into_iter()
      .sorted_by(compare_modules_by_identifier);

    let cgcb = self.get_chunk_graph_chunk_mut(*chunkb_ukey);
    let mut sorted_cgcb_modules_iter = cgcb
      .modules
      .clone()
      .into_iter()
      .sorted_by(compare_modules_by_identifier);

    compare_modules_by_identifier_iter(&mut sorted_cgca_modules_iter, &mut sorted_cgcb_modules_iter)
  }

  pub fn can_chunks_be_integrated(
    &self,
    chunk_group_by_ukey: &ChunkGroupByUkey,
    chunk_a: &Chunk,
    chunk_b: &Chunk,
  ) -> bool {
    if chunk_a.prevent_integration || chunk_b.prevent_integration {
      return false;
    }

    let has_runtime_a = chunk_a.has_runtime(&chunk_group_by_ukey);
    let has_runtime_b = chunk_b.has_runtime(&chunk_group_by_ukey);

    if has_runtime_a != has_runtime_b {
      if has_runtime_a {
        return self.is_available_chunk(chunk_group_by_ukey, chunk_a, chunk_b);
      } else if has_runtime_b {
        return self.is_available_chunk(chunk_group_by_ukey, chunk_b, chunk_a);
      } else {
        return false;
      }
    }

    if self.get_number_of_entry_modules(&chunk_a.ukey) > 0
      || self.get_number_of_entry_modules(&chunk_b.ukey) > 0
    {
      return false;
    }
    true
  }

  // TODO test it
  pub fn integrate_chunks(
    &mut self,
    module_graph: &ModuleGraph,
    chunk_group_by_ukey: &mut ChunkGroupByUkey,
    chunk_a: &mut Chunk,
    chunk_b: &mut Chunk,
  ) {
    if let (Some(chunk_a_name), Some(chunk_b_name)) = (&chunk_a.name, &chunk_b.name) {
      if !chunk_a_name.is_empty() && !chunk_b_name.is_empty() {
        if (self.get_number_of_entry_modules(&chunk_a.ukey) > 0)
          == (self.get_number_of_entry_modules(&chunk_b.ukey) > 0)
        {
          if chunk_a_name.len() != chunk_b_name.len() {
            chunk_a.name = if chunk_a_name.len() < chunk_b_name.len() {
              chunk_a.name.take()
            } else {
              chunk_b.name.take()
            };
          } else if chunk_a.name < chunk_b.name {
            chunk_a.name = chunk_a.name.take();
          } else {
            chunk_a.name = chunk_b.name.take();
          }
        }
      }
    } else if let Some(chunk_b_name) = &chunk_b.name {
      if !chunk_b_name.is_empty() {
        chunk_a.name = chunk_b.name.take();
      }
    }

    for hint in chunk_b.id_name_hints.iter() {
      chunk_a.id_name_hints.insert(hint.clone());
    }

    chunk_a.runtime = merge_runtime(chunk_a.runtime.clone(), chunk_b.runtime.clone());

    self
      .get_chunk_modules(&chunk_b.ukey, &module_graph)
      .iter()
      .for_each(|module| {
        self.disconnect_chunk_and_module(&chunk_b.ukey, module.identifier());
        self.connect_chunk_and_module(chunk_a.ukey, module.identifier());
      });

    self
      .get_chunk_entry_modules_with_chunk_group_iterable(&chunk_b.ukey)
      .clone()
      .iter()
      .for_each(|(module_identifier, entry)| {
        self.disconnect_chunk_and_entry_module(chunk_b.ukey, *module_identifier);
        self.connect_chunk_and_entry_module(chunk_a.ukey, *module_identifier, *entry);
      });

    chunk_b
      .groups
      .clone()
      .into_iter()
      .for_each(|chunk_group_ukey| {
        let chunk_group = chunk_group_by_ukey
          .get_mut(&chunk_group_ukey)
          .expect("should have chunk group");
        chunk_group.replace_chunk(chunk_b.ukey, chunk_a.ukey);
        chunk_a.add_group(chunk_group.ukey);
        chunk_b.remove_group(chunk_group.ukey);
      });
  }
}
