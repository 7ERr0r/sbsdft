use std::alloc::GlobalAlloc;
use std::alloc::Layout;

#[derive(Debug)]
pub struct KWasmTracingAllocator<A>(pub A)
where
    A: GlobalAlloc;

unsafe impl<A> GlobalAlloc for KWasmTracingAllocator<A>
where
    A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let _align = layout.align();
        let pointer = self.0.alloc(layout);
        if size > 1024 * 64 {
            use crate::klog;
            klog!("alloc of {} B", size);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let _size = layout.size();
        let _align = layout.align();
        self.0.dealloc(pointer, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let _size = layout.size();
        let _align = layout.align();
        let pointer = self.0.alloc_zeroed(layout);
        pointer
    }

    unsafe fn realloc(&self, old_pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _old_size = layout.size();
        let _align = layout.align();
        let new_pointer = self.0.realloc(old_pointer, layout, new_size);
        new_pointer
    }
}
