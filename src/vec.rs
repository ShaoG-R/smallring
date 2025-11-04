/// Fixed-capacity vector with stack/heap optimization
/// 
/// 固定容量的向量，带有栈/堆优化
/// 
/// This type provides a fixed-capacity vector that stores data on the stack
/// when capacity is ≤ N, and on the heap when capacity > N.
/// Unlike SmallVec, this type does not support dynamic resizing.
/// 
/// 此类型提供固定容量的向量，当容量 ≤ N 时在栈上存储数据，
/// 当容量 > N 时在堆上存储数据。与 SmallVec 不同，此类型不支持动态调整大小。

use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};

/// Fixed-capacity vector that optimizes for small sizes
/// 
/// 为小尺寸优化的固定容量向量
/// 
/// # Type Parameters
/// - `T`: Element type
/// - `N`: Stack capacity threshold (elements stored on stack when capacity ≤ N)
/// 
/// # 类型参数
/// - `T`: 元素类型
/// - `N`: 栈容量阈值（当容量 ≤ N 时元素存储在栈上）
pub struct FixedVec<T, const N: usize> {
    /// Storage backend
    /// 
    /// 存储后端
    storage: Storage<T, N>,
    
    /// Current length
    /// 
    /// 当前长度
    len: usize,
    
    /// Total capacity
    /// 
    /// 总容量
    capacity: usize,
}

/// Storage backend - either stack or heap
/// 
/// 存储后端 - 栈或堆
enum Storage<T, const N: usize> {
    /// Stack storage for capacity ≤ N
    /// 
    /// 容量 ≤ N 时的栈存储
    Stack([MaybeUninit<T>; N]),
    
    /// Heap storage for capacity > N
    /// 
    /// 容量 > N 时的堆存储
    Heap(Box<[MaybeUninit<T>]>),
}

impl<T, const N: usize> FixedVec<T, N> {
    /// Create a new FixedVec with the specified capacity
    /// 
    /// 创建指定容量的新 FixedVec
    /// 
    /// If `capacity` ≤ N, uses stack storage.
    /// If `capacity` > N, uses heap storage.
    /// 
    /// 如果 `capacity` ≤ N，使用栈存储。
    /// 如果 `capacity` > N，使用堆存储。
    /// 
    /// # Parameters
    /// - `capacity`: The fixed capacity of the vector
    /// 
    /// # Returns
    /// A new FixedVec with the specified capacity and length 0
    /// 
    /// # 参数
    /// - `capacity`: 向量的固定容量
    /// 
    /// # 返回值
    /// 指定容量、长度为 0 的新 FixedVec
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let storage = if capacity <= N {
            // Use stack storage
            // 使用栈存储
            // SAFETY: MaybeUninit does not require initialization
            // 安全性：MaybeUninit 不需要初始化
            Storage::Stack(unsafe { MaybeUninit::uninit().assume_init() })
        } else {
            // Use heap storage
            // 使用堆存储
            let mut vec = Vec::with_capacity(capacity);
            unsafe {
                vec.set_len(capacity);
            }
            Storage::Heap(vec.into_boxed_slice())
        };
        
        Self {
            storage,
            len: 0,
            capacity,
        }
    }
    
    /// Get the capacity of the vector
    /// 
    /// 获取向量的容量
    #[inline]
    #[allow(unused)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    
    /// Get the current length of the vector
    /// 
    /// 获取向量的当前长度
    #[inline]
    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// Check if the vector is empty
    /// 
    /// 检查向量是否为空
    #[inline]
    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    /// Set the length of the vector
    /// 
    /// 设置向量的长度
    /// 
    /// # Safety
    /// - `new_len` must be ≤ capacity
    /// - Elements in 0..new_len must be properly initialized if new_len > old len
    /// 
    /// # 安全性
    /// - `new_len` 必须 ≤ capacity
    /// - 如果 new_len > old len，则 0..new_len 中的元素必须正确初始化
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity);
        self.len = new_len;
    }
    
    /// Get a pointer to the element at the specified index (unchecked, fast path)
    /// 
    /// 获取指定索引处元素的指针（无检查，快速路径）
    /// 
    /// # Safety
    /// - `index` must be < capacity
    /// 
    /// # 安全性
    /// - `index` 必须 < capacity
    #[inline(always)]
    pub unsafe fn get_unchecked_ptr(&self, index: usize) -> *const MaybeUninit<T> {
        match &self.storage {
            Storage::Stack(arr) => unsafe { arr.as_ptr().add(index) },
            Storage::Heap(boxed) => unsafe { boxed.as_ptr().add(index) },
        }
    }
    
    /// Get a mutable pointer to the element at the specified index (unchecked, fast path)
    /// 
    /// 获取指定索引处元素的可变指针（无检查，快速路径）
    /// 
    /// # Safety
    /// - `index` must be < capacity
    /// 
    /// # 安全性
    /// - `index` 必须 < capacity
    #[inline(always)]
    pub unsafe fn get_unchecked_mut_ptr(&mut self, index: usize) -> *mut MaybeUninit<T> {
        match &mut self.storage {
            Storage::Stack(arr) => unsafe { arr.as_mut_ptr().add(index) },
            Storage::Heap(boxed) => unsafe { boxed.as_mut_ptr().add(index) },
        }
    }
    
    /// Get a pointer to the element at the specified index
    /// 
    /// 获取指定索引处元素的指针
    /// 
    /// # Safety
    /// - `index` must be < capacity
    /// 
    /// # 安全性
    /// - `index` 必须 < capacity
    #[inline]
    unsafe fn get_ptr(&self, index: usize) -> *const MaybeUninit<T> {
        debug_assert!(index < self.capacity);
        unsafe { self.get_unchecked_ptr(index) }
    }
    
    /// Get a mutable pointer to the element at the specified index
    /// 
    /// 获取指定索引处元素的可变指针
    /// 
    /// # Safety
    /// - `index` must be < capacity
    /// 
    /// # 安全性
    /// - `index` 必须 < capacity
    #[inline]
    unsafe fn get_mut_ptr(&mut self, index: usize) -> *mut MaybeUninit<T> {
        debug_assert!(index < self.capacity);
        unsafe { self.get_unchecked_mut_ptr(index) }
    }
}

impl<T, const N: usize> Index<usize> for FixedVec<T, N> {
    type Output = MaybeUninit<T>;
    
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.capacity, "index out of bounds");
        unsafe { &*self.get_ptr(index) }
    }
}

impl<T, const N: usize> IndexMut<usize> for FixedVec<T, N> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.capacity, "index out of bounds");
        unsafe { &mut *self.get_mut_ptr(index) }
    }
}

// Note: FixedVec does NOT implement Drop because it stores MaybeUninit<T>.
// The caller is responsible for properly dropping initialized elements.
// 
// 注意：FixedVec 不实现 Drop，因为它存储 MaybeUninit<T>。
// 调用者负责正确释放已初始化的元素。

// Ensure FixedVec is Send and Sync if T is Send and Sync
// 如果 T 是 Send 和 Sync，则确保 FixedVec 也是 Send 和 Sync
unsafe impl<T: Send, const N: usize> Send for FixedVec<T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for FixedVec<T, N> {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_small_capacity_stack() {
        // Test stack allocation (capacity ≤ 32)
        let vec: FixedVec<i32, 32> = FixedVec::with_capacity(16);
        assert_eq!(vec.capacity(), 16);
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
        assert!(matches!(vec.storage, Storage::Stack(_)));
    }
    
    #[test]
    fn test_large_capacity_heap() {
        // Test heap allocation (capacity > 32)
        let vec: FixedVec<i32, 32> = FixedVec::with_capacity(64);
        assert_eq!(vec.capacity(), 64);
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
        assert!(matches!(vec.storage, Storage::Heap(_)));
    }
    
    #[test]
    fn test_exact_threshold() {
        // Test exact threshold (capacity = 32)
        let vec: FixedVec<i32, 32> = FixedVec::with_capacity(32);
        assert_eq!(vec.capacity(), 32);
        assert!(matches!(vec.storage, Storage::Stack(_)));
    }
    
    #[test]
    fn test_set_len() {
        let mut vec: FixedVec<i32, 32> = FixedVec::with_capacity(16);
        unsafe {
            vec.set_len(10);
        }
        assert_eq!(vec.len(), 10);
        assert!(!vec.is_empty());
    }
    
    #[test]
    fn test_index_access() {
        let mut vec: FixedVec<i32, 32> = FixedVec::with_capacity(8);
        
        unsafe {
            // Write to indices
            vec[0].as_mut_ptr().write(42);
            vec[1].as_mut_ptr().write(99);
            vec.set_len(2);
            
            // Read from indices
            assert_eq!(*vec[0].as_ptr(), 42);
            assert_eq!(*vec[1].as_ptr(), 99);
        }
    }
    
    #[test]
    fn test_index_mut() {
        let mut vec: FixedVec<i32, 32> = FixedVec::with_capacity(8);
        
        unsafe {
            vec[0].as_mut_ptr().write(10);
            vec[1].as_mut_ptr().write(20);
            vec.set_len(2);
            
            // Modify through mutable index
            *vec[0].as_mut_ptr() = 100;
            
            assert_eq!(*vec[0].as_ptr(), 100);
            assert_eq!(*vec[1].as_ptr(), 20);
        }
    }
    
    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_index_out_of_bounds() {
        let vec: FixedVec<i32, 32> = FixedVec::with_capacity(8);
        let _ = &vec[10];
    }
    
    // Note: test_drop_cleanup has been removed because FixedVec does not implement Drop.
    // FixedVec stores MaybeUninit<T> and the caller is responsible for managing element lifetimes.
    // 
    // 注意：test_drop_cleanup 已移除，因为 FixedVec 不实现 Drop。
    // FixedVec 存储 MaybeUninit<T>，调用者负责管理元素生命周期。
    
    #[test]
    fn test_stack_vs_heap() {
        // Verify that small capacity uses less memory (stack)
        let stack_vec: FixedVec<u8, 32> = FixedVec::with_capacity(32);
        let heap_vec: FixedVec<u8, 32> = FixedVec::with_capacity(64);
        
        // Stack storage should use inline array
        assert!(matches!(stack_vec.storage, Storage::Stack(_)));
        // Heap storage should use boxed slice
        assert!(matches!(heap_vec.storage, Storage::Heap(_)));
    }
    
    #[test]
    fn test_zero_capacity() {
        let vec: FixedVec<i32, 32> = FixedVec::with_capacity(0);
        assert_eq!(vec.capacity(), 0);
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
    }
}

