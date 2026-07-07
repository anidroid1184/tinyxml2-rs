//! Generational arena allocator for DOM nodes.
//!
//! This module provides [`Arena`] and [`NodeId`], the foundation of the DOM's
//! memory model. All XML nodes live inside an `Arena` owned by the `Document`,
//! mirroring TinyXML2's model where `XMLDocument` owns all nodes.
//!
//! # Design
//!
//! The arena uses **generational indices** for safety:
//!
//! - Each slot has a **generation counter** that increments on deallocation.
//! - A [`NodeId`] captures both the slot index and the generation at allocation time.
//! - Accessing a slot with a stale generation returns `None` instead of accessing
//!   freed memory — providing use-after-free detection without `unsafe`.
//!
//! # Performance
//!
//! - Allocation is O(1) amortized (pop from free list or push to vec).
//! - Deallocation is O(1) (push to free list, increment generation).
//! - Access is O(1) (index into vec + generation check).
//! - Memory is contiguous (`Vec<Slot<T>>`) for cache-friendly traversal.
// u32 index is an intentional design choice — 4 billion nodes is sufficient for any XML document.
#![allow(clippy::cast_possible_truncation)]

use alloc::vec::Vec;
use core::{fmt, iter, mem, slice};

/// A unique, generation-checked identifier for an item in an [`Arena`].
///
/// `NodeId` is `Copy` and can be freely stored, passed around, and compared.
/// It becomes invalid after the corresponding item is deallocated — accessing
/// it will return `None` rather than corrupted data.
///
/// # Safety
///
/// `NodeId` provides **logical safety** (stale IDs are detected) without
/// requiring `unsafe` code. The generation check prevents use-after-free at
/// the logical level, though it cannot prevent a malicious caller from
/// constructing arbitrary IDs.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    /// The index into the arena's slot vector.
    index: u32,
    /// The generation at allocation time.
    generation: u32,
}

impl NodeId {
    /// Returns the raw index. Used internally for arena access.
    #[inline]
    pub(crate) const fn index(self) -> usize {
        self.index as usize
    }

    /// Returns the generation. Used internally for validity checks.
    #[inline]
    #[allow(dead_code)]
    pub(crate) const fn generation(self) -> u32 {
        self.generation
    }

    /// Creates a `NodeId` from raw index and generation values.
    ///
    /// This is primarily intended for FFI boundaries where node IDs need to be
    /// converted to/from C-compatible representations.
    ///
    /// # Correctness
    ///
    /// The caller is responsible for providing valid index/generation pairs that
    /// were originally obtained from the arena. Using arbitrary values may result
    /// in accessing wrong nodes or getting `None` from arena lookups.
    #[inline]
    #[must_use]
    pub const fn from_raw_parts(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns the raw index and generation values of this `NodeId`.
    ///
    /// This is primarily intended for FFI boundaries where node IDs need to be
    /// converted to C-compatible representations.
    #[inline]
    #[must_use]
    pub const fn raw_parts(self) -> (u32, u32) {
        (self.index, self.generation)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({}g{})", self.index, self.generation)
    }
}

/// Internal slot in the arena. Either occupied with a value or vacant.
#[derive(Debug)]
#[allow(clippy::cast_possible_truncation)] // u32 index is intentional — 4B nodes is sufficient
enum Slot<T> {
    /// Slot contains a live value.
    Occupied(T),
    /// Slot is vacant. Stores the index of the next vacant slot in the free
    /// list (or `u32::MAX` if this is the last free slot).
    Vacant(u32),
}

/// A generational arena allocator.
///
/// Provides O(1) allocation, deallocation, and access with generation-based
/// use-after-free detection. All operations are safe Rust.
///
/// # Type Parameter
///
/// `T` is the type of values stored in the arena. For the DOM, this will be
/// `NodeData`.
///
/// # Examples
///
/// ```
/// use tinyxml2::arena::Arena;
///
/// let mut arena: Arena<String> = Arena::new();
///
/// // Allocate
/// let id = arena.alloc("hello".to_string());
/// assert_eq!(arena.get(id), Some(&"hello".to_string()));
///
/// // Deallocate
/// let removed = arena.dealloc(id);
/// assert_eq!(removed, Some("hello".to_string()));
///
/// // Stale ID returns None
/// assert_eq!(arena.get(id), None);
/// ```
#[derive(Debug)]
#[allow(clippy::cast_possible_truncation)] // u32 index is intentional — 4B nodes is sufficient
pub struct Arena<T> {
    /// The slot storage.
    slots: Vec<Slot<T>>,
    /// Generation counter per slot. Incremented on each deallocation.
    generations: Vec<u32>,
    /// Head of the free list. `u32::MAX` means the free list is empty.
    free_head: u32,
    /// Number of currently occupied slots.
    len: usize,
}

impl<T> Arena<T> {
    /// Creates a new, empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            generations: Vec::new(),
            free_head: u32::MAX,
            len: 0,
        }
    }

    /// Creates a new arena with the specified capacity pre-allocated.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            generations: Vec::with_capacity(capacity),
            free_head: u32::MAX,
            len: 0,
        }
    }

    /// Allocates a new slot and stores the value, returning its [`NodeId`].
    ///
    /// If a previously-deallocated slot is available, it will be reused.
    /// Otherwise, a new slot is appended.
    ///
    /// Time complexity: O(1) amortized.
    pub fn alloc(&mut self, value: T) -> NodeId {
        self.len += 1;

        if self.free_head == u32::MAX {
            // Append a new slot
            let index = self.slots.len();
            self.slots.push(Slot::Occupied(value));
            self.generations.push(0);

            NodeId {
                index: index as u32,
                generation: 0,
            }
        } else {
            // Reuse a free slot
            let index = self.free_head as usize;
            let next_free = match self.slots[index] {
                Slot::Vacant(next) => next,
                Slot::Occupied(_) => unreachable!("free list pointed to occupied slot"),
            };
            self.slots[index] = Slot::Occupied(value);
            self.free_head = next_free;

            NodeId {
                index: index as u32,
                generation: self.generations[index],
            }
        }
    }

    /// Deallocates the slot identified by `id`, returning the stored value.
    ///
    /// Returns `None` if the ID is invalid (wrong generation, out of bounds,
    /// or already deallocated).
    ///
    /// The slot's generation is incremented so that any remaining copies of
    /// this `NodeId` become stale.
    ///
    /// Time complexity: O(1).
    pub fn dealloc(&mut self, id: NodeId) -> Option<T> {
        let index = id.index();
        if index >= self.slots.len() || self.generations[index] != id.generation {
            return None;
        }

        match &self.slots[index] {
            Slot::Vacant(_) => return None,
            Slot::Occupied(_) => {}
        }

        // Swap out the occupied slot for a vacant one
        let old_slot = mem::replace(&mut self.slots[index], Slot::Vacant(self.free_head));
        self.free_head = index as u32;
        self.generations[index] = self.generations[index].wrapping_add(1);
        self.len -= 1;

        match old_slot {
            Slot::Occupied(value) => Some(value),
            Slot::Vacant(_) => unreachable!(),
        }
    }

    /// Returns a reference to the value at `id`, or `None` if the ID is invalid.
    ///
    /// Time complexity: O(1).
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&T> {
        let index = id.index();
        if index >= self.slots.len() || self.generations[index] != id.generation {
            return None;
        }
        match &self.slots[index] {
            Slot::Occupied(value) => Some(value),
            Slot::Vacant(_) => None,
        }
    }

    /// Returns a mutable reference to the value at `id`, or `None` if invalid.
    ///
    /// Time complexity: O(1).
    #[must_use]
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        let index = id.index();
        if index >= self.slots.len() || self.generations[index] != id.generation {
            return None;
        }
        match &mut self.slots[index] {
            Slot::Occupied(value) => Some(value),
            Slot::Vacant(_) => None,
        }
    }

    /// Returns the number of currently occupied slots.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if no slots are currently occupied.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the total capacity (occupied + vacant slots).
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.slots.capacity()
    }

    /// Removes all items from the arena and resets all state.
    ///
    /// All existing `NodeId` values become invalid after this call.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.generations.clear();
        self.free_head = u32::MAX;
        self.len = 0;
    }

    /// Returns an iterator over all occupied `(NodeId, &T)` pairs.
    pub fn iter(&self) -> ArenaIter<'_, T> {
        ArenaIter {
            arena: self,
            index: 0,
        }
    }

    /// Returns a mutable iterator over all occupied `(NodeId, &mut T)` pairs.
    pub fn iter_mut(&mut self) -> ArenaIterMut<'_, T> {
        ArenaIterMut {
            inner: self.slots.iter_mut().enumerate(),
            generations: &self.generations,
        }
    }

    /// Returns `true` if the given `NodeId` currently refers to a live entry.
    #[must_use]
    pub fn contains(&self, id: NodeId) -> bool {
        self.get(id).is_some()
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = (NodeId, &'a T);
    type IntoIter = ArenaIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = (NodeId, &'a mut T);
    type IntoIter = ArenaIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Iterator over all occupied entries in an arena.
pub struct ArenaIter<'a, T> {
    arena: &'a Arena<T>,
    index: usize,
}

impl<'a, T> Iterator for ArenaIter<'a, T> {
    type Item = (NodeId, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.arena.slots.len() {
            let i = self.index;
            self.index += 1;
            if let Slot::Occupied(ref value) = self.arena.slots[i] {
                let id = NodeId {
                    index: i as u32,
                    generation: self.arena.generations[i],
                };
                return Some((id, value));
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.arena.slots.len() - self.index))
    }
}

/// Mutable iterator over all occupied entries in an arena.
pub struct ArenaIterMut<'a, T> {
    inner: iter::Enumerate<slice::IterMut<'a, Slot<T>>>,
    generations: &'a [u32],
}

impl<'a, T> Iterator for ArenaIterMut<'a, T> {
    type Item = (NodeId, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, slot) = self.inner.next()?;
            if let Slot::Occupied(value) = slot {
                let id = NodeId {
                    index: i as u32,
                    generation: self.generations[i],
                };
                return Some((id, value));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn alloc_and_get() {
        let mut arena = Arena::new();
        let id = arena.alloc(42);
        assert_eq!(arena.get(id), Some(&42));
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn alloc_multiple() {
        let mut arena = Arena::new();
        let id1 = arena.alloc("a");
        let id2 = arena.alloc("b");
        let id3 = arena.alloc("c");

        assert_eq!(arena.get(id1), Some(&"a"));
        assert_eq!(arena.get(id2), Some(&"b"));
        assert_eq!(arena.get(id3), Some(&"c"));
        assert_eq!(arena.len(), 3);
    }

    #[test]
    fn dealloc_returns_value() {
        let mut arena = Arena::new();
        let id = arena.alloc("hello".to_string());
        let removed = arena.dealloc(id);
        assert_eq!(removed, Some("hello".to_string()));
        assert_eq!(arena.len(), 0);
    }

    #[test]
    fn stale_id_returns_none() {
        let mut arena = Arena::new();
        let id = arena.alloc(42);
        arena.dealloc(id);
        assert_eq!(arena.get(id), None);
    }

    #[test]
    fn dealloc_stale_id_returns_none() {
        let mut arena = Arena::new();
        let id = arena.alloc(42);
        arena.dealloc(id);
        assert_eq!(arena.dealloc(id), None);
    }

    #[test]
    fn free_list_reuse() {
        let mut arena = Arena::new();
        let id1 = arena.alloc(1);
        let id2 = arena.alloc(2);

        // Dealloc id1, then alloc — should reuse id1's slot
        arena.dealloc(id1);
        let id3 = arena.alloc(3);

        // id3 should use the same index as id1 but with bumped generation
        assert_eq!(id3.index(), id1.index());
        assert_ne!(id3.generation(), id1.generation());
        assert_eq!(arena.get(id3), Some(&3));
        assert_eq!(arena.get(id1), None); // stale
        assert_eq!(arena.get(id2), Some(&2));
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn get_mut() {
        let mut arena = Arena::new();
        let id = arena.alloc(10);
        *arena.get_mut(id).unwrap() = 20;
        assert_eq!(arena.get(id), Some(&20));
    }

    #[test]
    fn get_mut_stale() {
        let mut arena = Arena::new();
        let id = arena.alloc(10);
        arena.dealloc(id);
        assert_eq!(arena.get_mut(id), None);
    }

    #[test]
    fn clear() {
        let mut arena = Arena::new();
        let id1 = arena.alloc(1);
        let id2 = arena.alloc(2);
        arena.clear();

        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
        assert_eq!(arena.get(id1), None);
        assert_eq!(arena.get(id2), None);
    }

    #[test]
    fn is_empty() {
        let mut arena: Arena<i32> = Arena::new();
        assert!(arena.is_empty());

        let id = arena.alloc(1);
        assert!(!arena.is_empty());

        arena.dealloc(id);
        assert!(arena.is_empty());
    }

    #[test]
    fn contains() {
        let mut arena = Arena::new();
        let id = arena.alloc(42);
        assert!(arena.contains(id));

        arena.dealloc(id);
        assert!(!arena.contains(id));
    }

    #[test]
    fn with_capacity() {
        let arena: Arena<i32> = Arena::with_capacity(100);
        assert!(arena.capacity() >= 100);
        assert!(arena.is_empty());
    }

    #[test]
    fn iter_all_occupied() {
        let mut arena = Arena::new();
        let id1 = arena.alloc(10);
        let id2 = arena.alloc(20);
        let id3 = arena.alloc(30);

        let items: Vec<_> = arena.iter().collect();
        assert_eq!(items.len(), 3);
        assert!(items.contains(&(id1, &10)));
        assert!(items.contains(&(id2, &20)));
        assert!(items.contains(&(id3, &30)));
    }

    #[test]
    fn iter_with_holes() {
        let mut arena = Arena::new();
        let id1 = arena.alloc(10);
        let id2 = arena.alloc(20);
        let id3 = arena.alloc(30);

        arena.dealloc(id2);

        let items: Vec<_> = arena.iter().collect();
        assert_eq!(items.len(), 2);
        assert!(items.contains(&(id1, &10)));
        assert!(items.contains(&(id3, &30)));
    }

    #[test]
    fn iter_empty() {
        let arena: Arena<i32> = Arena::new();
        assert_eq!(arena.iter().count(), 0);
    }

    #[test]
    fn out_of_bounds_id() {
        let arena: Arena<i32> = Arena::new();
        let fake_id = NodeId {
            index: 999,
            generation: 0,
        };
        assert_eq!(arena.get(fake_id), None);
    }

    #[test]
    fn generation_wrapping() {
        let mut arena = Arena::new();

        // Allocate and deallocate the same slot many times
        let mut last_id = arena.alloc(0);
        for i in 1..10 {
            arena.dealloc(last_id);
            last_id = arena.alloc(i);
        }

        // The latest ID should work
        assert_eq!(arena.get(last_id), Some(&9));
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn node_id_debug() {
        let id = NodeId {
            index: 5,
            generation: 3,
        };
        assert_eq!(format!("{id:?}"), "NodeId(5g3)");
    }

    #[test]
    fn default_arena() {
        let arena: Arena<i32> = Arena::default();
        assert!(arena.is_empty());
    }

    #[test]
    fn stress_alloc_dealloc() {
        let mut arena = Arena::new();
        let mut ids = Vec::new();

        // Alloc 1000 items
        for i in 0..1000 {
            ids.push(arena.alloc(i));
        }
        assert_eq!(arena.len(), 1000);

        // Dealloc every other one
        for i in (0..1000).step_by(2) {
            arena.dealloc(ids[i]);
        }
        assert_eq!(arena.len(), 500);

        // Alloc 500 more (should reuse freed slots)
        for i in 0..500 {
            arena.alloc(i + 1000);
        }
        assert_eq!(arena.len(), 1000);
    }

    #[test]
    fn node_id_is_copy_and_eq() {
        let id1 = NodeId {
            index: 0,
            generation: 0,
        };
        let id2 = id1; // Copy
        assert_eq!(id1, id2);
    }

    #[test]
    #[cfg(feature = "std")]
    fn node_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        let id1 = NodeId {
            index: 0,
            generation: 0,
        };
        let id2 = NodeId {
            index: 1,
            generation: 0,
        };
        set.insert(id1);
        set.insert(id2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&id1));
    }
}
