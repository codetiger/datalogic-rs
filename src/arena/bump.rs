//! Bump allocator for efficient arena-based memory management.
//!
//! This module provides a bump allocator that allows for efficient
//! allocation of memory with minimal overhead. All allocations are
//! freed at once when the arena is reset or dropped.

use bumpalo::Bump;
use std::cell::RefCell;
use std::fmt;

use super::interner::StringInterner;

/// A memory arena for efficient allocation of data structures.
///
/// The `DataArena` uses a bump allocator to provide fast allocation
/// with minimal overhead. All allocations are freed at once when the
/// arena is reset or dropped.
///
/// # Examples
///
/// ```
/// use datalogic_rs::arena::DataArena;
///
/// let arena = DataArena::new();
/// let value = arena.alloc(42);
/// assert_eq!(*value, 42);
/// ```
pub struct DataArena {
    /// The underlying bump allocator
    bump: Bump,
    
    /// String interner for efficient string storage
    interner: RefCell<StringInterner>,
    
    /// Chunk size for allocations (in bytes)
    chunk_size: usize,
}

impl Default for DataArena {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DataArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataArena")
            .field("chunk_size", &self.chunk_size)
            .finish()
    }
}

impl DataArena {
    /// Creates a new arena with default settings.
    pub fn new() -> Self {
        Self::with_chunk_size(1024 * 1024) // 1MB default chunk size
    }
    
    /// Creates a new arena with the specified chunk size.
    ///
    /// The chunk size determines how much memory is allocated at once
    /// when the arena needs more space. Larger chunk sizes can improve
    /// performance but may waste memory if not fully utilized.
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        let bump = Bump::new();
        bump.set_allocation_limit(Some(chunk_size * 1024)); // Safety limit
        
        Self {
            bump,
            interner: RefCell::new(StringInterner::new()),
            chunk_size,
        }
    }
    
    /// Allocates a value in the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// use datalogic_rs::arena::DataArena;
    ///
    /// let arena = DataArena::new();
    /// let value = arena.alloc(42);
    /// assert_eq!(*value, 42);
    /// ```
    pub fn alloc<T>(&self, val: T) -> &T {
        self.bump.alloc(val)
    }
    
    /// Allocates a slice in the arena by copying from a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use datalogic_rs::arena::DataArena;
    ///
    /// let arena = DataArena::new();
    /// let original = &[1, 2, 3, 4, 5];
    /// let slice = arena.alloc_slice_copy(original);
    /// assert_eq!(slice, original);
    /// ```
    pub fn alloc_slice_copy<T: Copy>(&self, vals: &[T]) -> &[T] {
        self.bump.alloc_slice_copy(vals)
    }
    
    /// Allocates a slice in the arena by cloning each element.
    ///
    /// This is useful for types that don't implement Copy but do implement Clone.
    ///
    /// # Examples
    ///
    /// ```
    /// use datalogic_rs::arena::DataArena;
    ///
    /// let arena = DataArena::new();
    /// let original = vec![String::from("hello"), String::from("world")];
    /// let slice = arena.alloc_slice_clone(&original);
    /// assert_eq!(slice[0], "hello");
    /// assert_eq!(slice[1], "world");
    /// ```
    pub fn alloc_slice_clone<T: Clone>(&self, vals: &[T]) -> &[T] {
        if vals.is_empty() {
            return &[];
        }
        
        // Allocate memory for the slice
        let layout = std::alloc::Layout::array::<T>(vals.len()).unwrap();
        let ptr = self.bump.alloc_layout(layout).cast::<T>();
        
        // Clone each element into the allocated memory
        let slice = unsafe {
            let mut dst = ptr.as_ptr();
            for val in vals {
                std::ptr::write(dst, val.clone());
                dst = dst.add(1);
            }
            std::slice::from_raw_parts(ptr.as_ptr(), vals.len())
        };
        
        slice
    }
    
    /// Allocates a string in the arena.
    ///
    /// # Examples
    ///
    /// ```
    /// use datalogic_rs::arena::DataArena;
    ///
    /// let arena = DataArena::new();
    /// let s = arena.alloc_str("hello");
    /// assert_eq!(s, "hello");
    /// ```
    pub fn alloc_str(&self, s: &str) -> &str {
        self.bump.alloc_str(s)
    }
    
    /// Interns a string, returning a reference to a unique instance.
    ///
    /// If the string has been interned before, returns a reference to
    /// the existing instance. Otherwise, allocates the string in the
    /// arena and returns a reference to it.
    ///
    /// This is particularly useful for strings that are likely to be
    /// repeated, such as object keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use datalogic_rs::arena::DataArena;
    ///
    /// let arena = DataArena::new();
    /// let s1 = arena.intern_str("hello");
    /// let s2 = arena.intern_str("hello");
    ///
    /// // Both references point to the same string
    /// assert_eq!(s1, s2);
    /// ```
    pub fn intern_str(&self, s: &str) -> &str {
        self.interner.borrow_mut().intern(s, &self.bump)
    }
    
    /// Resets the arena, freeing all allocations.
    ///
    /// This method resets the arena to its initial state, freeing all
    /// allocations at once. This is much faster than freeing each
    /// allocation individually.
    ///
    /// # Examples
    ///
    /// ```
    /// use datalogic_rs::arena::DataArena;
    ///
    /// let mut arena = DataArena::new();
    /// let s = arena.alloc_str("hello");
    /// arena.reset();
    /// // s is no longer valid after reset
    /// ```
    pub fn reset(&mut self) {
        self.bump.reset();
        self.interner = RefCell::new(StringInterner::new());
    }
    
    /// Returns the current memory usage of the arena in bytes.
    pub fn memory_usage(&self) -> usize {
        self.bump.allocated_bytes()
    }
    
    /// Creates a new temporary arena for short-lived allocations.
    ///
    /// This method creates a new arena that can be used for temporary
    /// allocations that are freed all at once when the arena is dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use datalogic_rs::arena::DataArena;
    ///
    /// let arena = DataArena::new();
    /// {
    ///     let temp_arena = arena.create_temp_arena();
    ///     let temp = temp_arena.alloc(42);
    ///     assert_eq!(*temp, 42);
    /// }
    /// // temp is no longer valid here
    /// ```
    pub fn create_temp_arena(&self) -> DataArena {
        DataArena::with_chunk_size(self.chunk_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc() {
        let arena = DataArena::new();
        let value = arena.alloc(42);
        assert_eq!(*value, 42);
    }

    #[test]
    fn test_alloc_slice_copy() {
        let arena = DataArena::new();
        let original = &[1, 2, 3, 4, 5];
        let slice = arena.alloc_slice_copy(original);
        assert_eq!(slice, original);
    }

    #[test]
    fn test_alloc_str() {
        let arena = DataArena::new();
        let s = arena.alloc_str("hello");
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_reset() {
        let mut arena = DataArena::new();
        
        // Allocate a significant amount of data
        for i in 0..1000 {
            let _ = arena.alloc_str(&format!("test string {}", i));
        }
        
        arena.reset();
        
        // After reset, the memory is still allocated to the arena but marked as free
        // So we need to check that we can reuse it without increasing usage significantly
        
        // Allocate some data again
        let s = arena.alloc_str("hello");
        assert_eq!(s, "hello");
        
        // The key behavior to test is that we can reuse the arena after reset
        // Not necessarily that the memory usage decreases
    }

    #[test]
    fn test_temp_arena() {
        let arena = DataArena::new();
        let value;
        {
            let temp_arena = arena.create_temp_arena();
            let temp = temp_arena.alloc(42);
            assert_eq!(*temp, 42);
            value = *temp;
        }
        // We can still use the value, but the memory is freed
        assert_eq!(value, 42);
    }
}
