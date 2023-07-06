/*
 * File:    main.rs
 * Brief:   TODO
 *
 * Copyright: Copyright (C) 2023 John Jekel
 * See the LICENSE file at the root of the project for licensing info.
 *
 * TODO longer description
 *
 * Thanks in part to: https://github.com/spencertipping/jit-tutorial
 * And: https://www.jntrnr.com/building-a-simple-jit-in-rust/
 *
*/

/*!
 * TODO rustdoc for this file here
*/

/* ------------------------------------------------------------------------------------------------
 * Submodules
 * --------------------------------------------------------------------------------------------- */

//TODO (includes "mod ..." and "pub mod ...")

/* ------------------------------------------------------------------------------------------------
 * Uses
 * --------------------------------------------------------------------------------------------- */

//TODO (includes "use ..." and "extern crate ...")

/* ------------------------------------------------------------------------------------------------
 * Macros
 * --------------------------------------------------------------------------------------------- */

//TODO (also pub(crate) use the_macro statements here too)

/* ------------------------------------------------------------------------------------------------
 * Constants
 * --------------------------------------------------------------------------------------------- */

const PAGE_SIZE: usize = 4096;

/* ------------------------------------------------------------------------------------------------
 * Static Variables
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Types
 * --------------------------------------------------------------------------------------------- */

struct RWXMemory<'a> {
    memory: &'a mut [u8]//TODO is this okay to do with lifetimes?
}

//TODO perhaps instead in the future we should store a slice internally, which will keep track of
//the length for us, and we can basically do an "Executable owned slice" by rounding up the length
//to the nearest multiple of the page size, and then mmaping that many pages as executable

#[repr(transparent)]
struct RWXPage {
    page_ptr: std::ptr::NonNull<u8>,
}

//TODO for safety never allow W+X?

/*
#[repr(transparent)]
struct RXPage {
    page_ptr: std::ptr::NonNull<u8>,
}
*/

struct JITPage {
    page: RWXPage,
    //groups: Vec<std::ptr::NonNull<u8>>,
}

/* ------------------------------------------------------------------------------------------------
 * Associated Functions and Methods
 * --------------------------------------------------------------------------------------------- */

impl RWXMemory<'_> {
    pub fn new(size_bytes: usize) -> Option<Self> {//Guaranteed to get at least this size (but may be larger)
        //Determine the length to pass to mmap()
        if size_bytes == 0 {
            //Not allowed to mmap() a zero length region
            return None;
        } else if size_bytes > isize::MAX as usize {
            //Requirement on Rust slices
            return None;
        }

        let mem_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size_bytes,
                libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            )
        };

        if mem_ptr == libc::MAP_FAILED {
            None
        } else if mem_ptr.is_null() {
            //mmap() could return a pointer to page 0 in rare circumstances
            //We don't support this case
            unsafe { libc::munmap(mem_ptr, size_bytes); }
            None
        } else {
            //Zero out the memory just in case we're on a platform that didn't do this
            unsafe { std::ptr::write_bytes(mem_ptr, 0, size_bytes); }

            Some(
                RWXMemory {
                    memory: unsafe { std::slice::from_raw_parts_mut(mem_ptr as *mut u8, size_bytes) }
                }
            )
        }
    }
}

impl RWXPage {
    fn new() -> Option<Self> {
        let page_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                PAGE_SIZE,
                libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            )
        };

        if page_ptr == libc::MAP_FAILED {
            None
        } else if page_ptr.is_null() {
            //mmap() could return a pointer to page 0 in rare circumstances
            //We don't support this case
            unsafe { libc::munmap(page_ptr, PAGE_SIZE); }
            None
        } else {
            Some(
                RWXPage {
                    page_ptr: std::ptr::NonNull::new(page_ptr).unwrap().cast()
                }
            )
        }
    }

    //THIS DOES NOT TAKE OWNERSHIP; YOU MUST NOT MUNMAP THE PAGE YOURSELF
    fn as_ptr(&self) -> std::ptr::NonNull<u8> {
        self.page_ptr
    }

    //THIS DOES TAKE OWNERSHIP; YOU MUST MUNMAP THE PAGE YOURSELF (to avoid a memory leak)
    fn take_ptr(self) -> std::ptr::NonNull<u8> {
        let ptr = self.page_ptr;
        std::mem::forget(self);
        ptr
    }
}

impl JITPage {
    fn new() -> Option<Self> {
        let jitpage = JITPage {
                page: RWXPage::new()?,
        };

        //Initialize the page
        unsafe {
            let page_ptr = jitpage.page.as_ptr().as_ptr();

            //Fill the page with a nop slide into an illegal opcode to help catch bugs
            //IT IS EXPECTED THAT THE BYTES THAT ARE WRITTEN RETURN values that make sense for you
            for i in 0..PAGE_SIZE {
                *page_ptr.add(i) = 0x90;
            }
            debug_assert!(PAGE_SIZE >= 2);
            *page_ptr.add(PAGE_SIZE - 2) = 0x0F;
            *page_ptr.add(PAGE_SIZE - 1) = 0xFF;
        }
        
        Some(jitpage)
    }

    fn add_byte_group(&mut self, bytes: &[u8]) -> Result<(), ()> {
        //TODO this is only successful if the instruction fits in the page in the space we have left
        //TODO perhaps dynamically increase the page size if we run out of space?
        //TODO also keep track of the locations of each byte group which could be useful if
        //we wish to jump to a specific byte group
        //TODO also 
        Err(())
    }

    //Transmute these functions yourself to the correct function pointer type for you
    //THIS DOES NOT TAKE OWNERSHIP; YOU MUST NOT MUNMAP THE PAGE YOURSELF
    //TODO add unsafe macros to do the transmute

    fn get_ptr_to_start(&self) -> std::ptr::NonNull<u8> {//Equivalent to execute_at_byte_group(0), but faster
        self.page.as_ptr()
    }

    fn get_ptr_to_byte_group(&self, group: usize) -> std::ptr::NonNull<u8> {
        todo!()
    }
}

/* ------------------------------------------------------------------------------------------------
 * Traits And Default Implementations
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Trait Implementations
 * --------------------------------------------------------------------------------------------- */

impl Drop for RWXMemory<'_> {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.memory.as_mut_ptr().cast(), self.memory.len());
        }
    }
}

impl AsRef<[u8]> for RWXMemory<'_> {
    fn as_ref(&self) -> &[u8] {
        self.memory
    }
}

impl AsMut<[u8]> for RWXMemory<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.memory
    }
}

impl Drop for RWXPage {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.page_ptr.as_ptr().cast(), PAGE_SIZE);
        }
    }
}

/* ------------------------------------------------------------------------------------------------
 * Functions
 * --------------------------------------------------------------------------------------------- */

fn main() {
    println!("Hello, world!");

    let jitpage = JITPage::new().expect("Failed to allocate a page of memory");

    unsafe {
        let start_fn_ptr: unsafe fn() = std::mem::transmute(jitpage.get_ptr_to_start().as_ptr());
        start_fn_ptr();
    }

    
    //let rwxpage = RWXPage::new().expect("Failed to allocate a page of memory");
    //let ptr = rwpage.as_ptr();
    //let owned_ptr = rwpage.take_ptr();

    //Initial experiments
    let page_ptr = allocate_rwx_page().expect("Failed to allocate a page of memory");
    let jit_function = jit(page_ptr);
    println!("jit_function() returned {}", jit_function());
    free_rwx_page(page_ptr);
}

fn jit(rwx_page_ptr_to_use: std::ptr::NonNull<std::ffi::c_void>) -> fn() -> i32 {
    let page_ptr = rwx_page_ptr_to_use.as_ptr() as *mut u8;
    
    unsafe {
        //mov rax, 0x3
        *page_ptr.offset(0) = 0x48;
        *page_ptr.offset(1) = 0xC7;
        *page_ptr.offset(2) = 0xC0;
        *page_ptr.offset(3) = 0x03;
        *page_ptr.offset(4) = 0x00;
        *page_ptr.offset(5) = 0x00;
        *page_ptr.offset(6) = 0x00;

        //ret
        *page_ptr.offset(7) = 0xC3;

        std::mem::transmute(page_ptr)
    }
}

fn allocate_rwx_page() -> Option<std::ptr::NonNull<std::ffi::c_void>> {
    let page_ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            4096,
            libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
            libc::MAP_ANON | libc::MAP_PRIVATE,
            -1,
            0,
        )
    };

    if page_ptr == libc::MAP_FAILED {
        None
    } else {
        Some(std::ptr::NonNull::new(page_ptr).expect("We don't handle the case where mmap allocates a page for us at address 0"))
    }
}

fn free_rwx_page(page_ptr: std::ptr::NonNull<std::ffi::c_void>) {
    unsafe {
        libc::munmap(page_ptr.as_ptr(), 4096);
    }
}

/* ------------------------------------------------------------------------------------------------
 * Tests
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Benchmarks
 * --------------------------------------------------------------------------------------------- */

//TODO
