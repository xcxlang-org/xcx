use std::cell::RefCell;
use std::ptr::NonNull;

pub struct Arena {
    chunks: RefCell<Vec<Vec<u8>>>,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            chunks: RefCell::new(vec![Vec::with_capacity(4096)]),
        }
    }

    pub fn alloc<T>(&self, value: T) -> *mut T {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();
        
        let mut chunks = self.chunks.borrow_mut();
        let last_chunk = chunks.last_mut().unwrap();
        
        let current_ptr = last_chunk.as_ptr() as usize + last_chunk.len();
        let aligned_ptr = (current_ptr + align - 1) & !(align - 1);
        let padding = aligned_ptr - current_ptr;
        
        if last_chunk.len() + padding + size > last_chunk.capacity() {
            let new_cap = (size + 4095) & !4095;
            chunks.push(Vec::with_capacity(new_cap.max(4096)));
            return self.alloc(value);
        }
        
        last_chunk.extend(std::iter::repeat(0).take(padding));
        let start = last_chunk.len();
        last_chunk.extend(std::iter::repeat(0).take(size));
        
        let ptr = unsafe { last_chunk.as_mut_ptr().add(start) as *mut T };
        unsafe { std::ptr::write(ptr, value); }
        ptr
    }
}

thread_local! {
    pub static CURRENT_ARENA: RefCell<Option<NonNull<Arena>>> = RefCell::new(None);
}

pub fn with_arena<F, R>(arena: &Arena, f: F) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_ARENA.with(|a| {
        let old = a.replace(Some(NonNull::from(arena)));
        let res = f();
        *a.borrow_mut() = old;
        res
    })
}

pub fn alloc_in_arena<T>(value: T) -> Option<*mut T> {
    CURRENT_ARENA.with(|a| {
        a.borrow().map(|arena_ptr| {
            let arena = unsafe { arena_ptr.as_ref() };
            arena.alloc(value)
        })
    })
}
