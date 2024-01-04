use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

pub struct BumpAllocator {
    mem: Mutex<Vec<u8, System>>,
    pos: AtomicUsize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            mem: Mutex::new(Vec::new_in(System)),
            pos: AtomicUsize::new(0),
        }
    }

    pub fn init(&self, ballast_size: usize) {
        self.mem.lock().unwrap().resize_with(ballast_size, || 0);
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let amount = layout.size() + layout.align();
        let current_offset = self.pos.fetch_add(amount, Ordering::Relaxed);
        let mut mem = self.mem.lock().unwrap();

        let ptr = mem
            .get_mut(current_offset)
            .map(|value| value as *mut _)
            .unwrap();

        ptr
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}

#[cfg(test)]
mod tests {
    use super::BumpAllocator;
    use std::alloc::{GlobalAlloc, Layout};

    #[test]
    fn bump_alloc() {
        let balloc = BumpAllocator::new();
        balloc.init(8);

        assert_eq!(balloc.mem.lock().unwrap().capacity(), 8);

        let first_addr = balloc.mem.lock().unwrap().first().unwrap() as *const u8;

        let first_block = unsafe { balloc.alloc(Layout::for_value(&1u8)) };
        let second_block = unsafe { balloc.alloc(Layout::for_value(&1u16)) };
        let third_block = unsafe { balloc.alloc(Layout::for_value(&1u32)) };

        assert_eq!(first_block.addr(), first_addr.addr());
        assert_eq!(second_block.addr(), first_addr.addr() + 1);
        assert_eq!(third_block.addr(), first_addr.addr() + 3);
    }
}
