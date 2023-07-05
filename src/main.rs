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

//TODO

/* ------------------------------------------------------------------------------------------------
 * Static Variables
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Types
 * --------------------------------------------------------------------------------------------- */

//TODO includes "type"-defs, structs, enums, unions, etc

/* ------------------------------------------------------------------------------------------------
 * Associated Functions and Methods
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Traits And Default Implementations
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Trait Implementations
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Functions
 * --------------------------------------------------------------------------------------------- */

fn main() {
    println!("Hello, world!");

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
