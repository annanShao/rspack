use std::cmp::Ordering;
use std::vec::IntoIter;

use itertools::Itertools; // TODO delete if useless
use rspack_identifier::Identifier;
// use std::collections::hash_map::DefaultHasher;
// use std::hash::{Hash, Hasher};
// use std::sync::{Arc, Mutex};

// use rustc_hash::FxHashMap;

// type ParameterizedComparator<P> = Arc<dyn for<'a> Fn(&'a P, &'a P) -> Ordering + Send + Sync>;

// type ParameterizedComparatorWrapper<T, P> = Arc<dyn Fn(&T) -> ParameterizedComparator<P>>;

// fn create_cached_parameterized_comparator<T, F, P: 'static>(
//   comparator: F,
// ) -> ParameterizedComparatorWrapper<T, P>
// where
//   T: 'static + Hash + Sync,
//   F: 'static + Fn(&T, &P, &P) -> Ordering + Sync,
// {
//   let map: Arc<Mutex<FxHashMap<u64, ParameterizedComparator<P>>>> =
//     Arc::new(Mutex::new(FxHashMap::default()));

//   Arc::new(move |arg: &T| {
//     let mut hasher = DefaultHasher::new();
//     arg.hash(&mut hasher);
//     let hash = hasher.finish();

//     let mut map = map.lock().unwrap();

//     if let Some(cached_result) = map.get(&hash) {
//       return Arc::clone(cached_result);
//     }

//     let result = Arc::new(|a: &P, b: &P| comparator(arg, a, b));

//     map.insert(
//       hash,
//       Arc::clone(&(result as Arc<dyn for<'a> Fn(&'a P, &'a P) -> Ordering + Send + Sync>)),
//     );

//     Arc::clone(&(result as Arc<dyn for<'a> Fn(&'a P, &'a P) -> Ordering + Send + Sync>))
//   })
// }

#[allow(clippy::comparison_chain)]
pub fn compare_ids(a: &str, b: &str) -> Ordering {
  let a = a.to_lowercase();
  let b = b.to_lowercase();
  if a < b {
    Ordering::Less
  } else if a > b {
    Ordering::Greater
  } else {
    Ordering::Equal
  }
}
#[allow(clippy::comparison_chain)]
pub fn compare_numbers(a: usize, b: usize) -> Ordering {
  if a < b {
    Ordering::Less
  } else if a > b {
    Ordering::Greater
  } else {
    Ordering::Equal
  }
}

#[allow(clippy::comparison_chain)]
pub fn compare_modules_by_identifier(a: &Identifier, b: &Identifier) -> Ordering {
  compare_ids(a.as_str(), b.as_str())
}

#[allow(clippy::comparison_chain)]
pub fn compare_modules_by_identifier_iter(
  a: &mut IntoIter<Identifier>,
  b: &mut IntoIter<Identifier>,
) -> Ordering {
  loop {
    let item_a = a.next();
    let item_b = b.next();

    if item_a.is_none() {
      if item_b.is_none() {
        return Ordering::Equal;
      } else {
        return Ordering::Less;
      }
    } else if item_b.is_none() {
      return Ordering::Greater;
    }

    let result = compare_modules_by_identifier(&item_a.unwrap(), &item_b.unwrap());
    if result != Ordering::Equal {
      return result;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_compare_ids() {
    assert_eq!(compare_ids("abc", "def"), Ordering::Less);
    assert_eq!(compare_ids("DEF", "abc"), Ordering::Greater);
    assert_eq!(compare_ids("abc", "ABC"), Ordering::Equal);
  }

  #[test]
  fn test_compare_numbers() {
    assert_eq!(compare_numbers(1, 2), Ordering::Less);
    assert_eq!(compare_numbers(2, 1), Ordering::Greater);
    assert_eq!(compare_numbers(1, 1), Ordering::Equal);
  }

  #[test]
  fn test_compare_modules_by_identifier() {
    assert_eq!(
      compare_modules_by_identifier(&Identifier::from("abc"), &Identifier::from("def")),
      Ordering::Less
    );
    assert_eq!(
      compare_modules_by_identifier(&Identifier::from("DEF"), &Identifier::from("abc")),
      Ordering::Greater
    );
    assert_eq!(
      compare_modules_by_identifier(&Identifier::from("abc"), &Identifier::from("ABC")),
      Ordering::Equal
    );
  }
}
