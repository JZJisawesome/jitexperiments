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
}

enum JITPageExecutionResult {
    EndOfPage,
    //TODO others
}

/* ------------------------------------------------------------------------------------------------
 * Associated Functions and Methods
 * --------------------------------------------------------------------------------------------- */

impl RWXPage {
    fn new() -> Option<Self> {
        let page_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                PAGE_SIZE,
                //libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                libc::PROT_READ | libc::PROT_WRITE,
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

            //Fill the page with amd64 nops (nop slide to the end)
            for i in 0..PAGE_SIZE {
                *page_ptr.add(i) = 0x90;
            }

            //Return 0 if we ever hit the end of the page
            debug_assert!(PAGE_SIZE >= 3);

            //Set rax to 0 (writing the lower 32 bits also clears the upper 32 bits)
            //xor eax, eax
            *page_ptr.add(PAGE_SIZE - 3) = 0x31;
            *page_ptr.add(PAGE_SIZE - 2) = 0xC0;

            //ret
            *page_ptr.add(PAGE_SIZE - 1) = 0xC3;
        }
        
        Some(jitpage)
    }

    fn add_byte_group(&mut self, instruction: &[u8]) -> Result<(), ()> {
        //TODO this is only successful if the instruction fits in the page in the space we have left
        //TODO perhaps dynamically increase the page size if we run out of space?
        //TODO also keep track of the locations of each byte group which could be useful if
        //we wish to jump to a specific byte group
        //TODO also 
        Err(())
    }

    fn execute_from_start(&self) -> JITPageExecutionResult {//Equivalent to execute_at_byte_group(0)
        let function_ptr = self.page.as_ptr().as_ptr().cast::<fn() -> u64>();
        JITPageExecutionResult::EndOfPage
    }

    fn execute_at_byte_group(&self, byte_group_index: usize) -> JITPageExecutionResult {
        //TODO
        JITPageExecutionResult::EndOfPage
    }
}

/* ------------------------------------------------------------------------------------------------
 * Traits And Default Implementations
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Trait Implementations
 * --------------------------------------------------------------------------------------------- */

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

    let rwxpage = RWXPage::new().expect("Failed to allocate a page of memory");
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
