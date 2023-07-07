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

mod amd64asm;
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

#[derive(Debug)]//We will NEVER dervie Copy or Clone. We may manually implement Clone in the future
//But this will never be Copy
struct RWXMemory {
    mem_ptr: std::ptr::NonNull<u8>,
    mem_len: usize
}//TODO add option to disable reading/writing/execution after creation? Or make them opt-in?

struct JITMemory {
    memory: RWXMemory,
    //Cannot keep slices to each group because we couldn't modify memory then!
    //Keep all of the end indices of each group instead (exclusive)
    group_ends: Vec<usize>
}

/* ------------------------------------------------------------------------------------------------
 * Associated Functions and Methods
 * --------------------------------------------------------------------------------------------- */

impl RWXMemory {
    pub fn new(size_bytes: usize) -> Option<Self> {//Guaranteed to get at least this size (but may be larger)
        //Determine the length to pass to mmap()
        if size_bytes == 0 {
            //Not allowed to mmap() a zero length region
            return None;
        } else if size_bytes > isize::MAX as usize {
            //Requirement on Rust slices
            return None;
        }

        let actual_size = size_bytes;//May need to change this in the future

        let mem_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                actual_size,
                libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            )
        };

        if mem_ptr == libc::MAP_FAILED {
            None
        } else if mem_ptr.is_null() {
            //mmap() could return a pointer to page 0 in rare circumstances. We don't support this
            unsafe { libc::munmap(mem_ptr, actual_size); }
            None
        } else {
            //Zero out the memory just in case we're on a platform that didn't do this
            unsafe { std::ptr::write_bytes(mem_ptr, 0x00, size_bytes); }

            Some(
                RWXMemory {
                    mem_ptr: std::ptr::NonNull::new(mem_ptr).expect("We already checked for null").cast(),
                    mem_len: actual_size,
                }
            )
        }
    }

    //THIS DOES NOT TAKE OWNERSHIP; YOU MUST NOT MUNMAP THE PAGE YOURSELF
    //Kind of redundant since the slices provide as_ptr(), so we comment it out
    /*
    fn as_ptr(&self) -> std::ptr::NonNull<u8> {
        self.mem_ptr
    }
    */

    //THIS DOES TAKE OWNERSHIP; YOU MUST MUNMAP THE PAGE YOURSELF (to avoid a memory leak)
    fn into_raw(self) -> std::ptr::NonNull<u8> {
        let ptr = self.mem_ptr;
        std::mem::forget(self);
        ptr
    }

    //TODO add from_raw function (will also need the length)
    unsafe fn from_raw(ptr: std::ptr::NonNull<u8>, len: usize) -> Self {
        RWXMemory {
            mem_ptr: ptr,
            mem_len: len
        }
    }
}

impl JITMemory {
    fn new(size_bytes: usize) -> Option<Self> {
        Some(
            JITMemory {
                memory: RWXMemory::new(size_bytes)?,
                group_ends: Vec::new()
            }
        )
    }

    fn len(&self) -> usize {
        self.memory.len()
    }

    fn remaining_space(&self) -> usize {
        if self.group_ends.is_empty() {
            self.len()
        } else {
            self.len() - self.group_ends.last().expect("We already checked that group_ends is not empty")
        }
    }

    fn num_byte_groups(&self) -> usize {
        self.group_ends.len()
    }

    fn add_byte_group(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if self.remaining_space() < bytes.len() {
            Err(())
        } else {
            let group_start_index;//Inclusive
            if self.group_ends.is_empty() {
                group_start_index = 0;
            } else {
                group_start_index = *self.group_ends.last().expect("We already checked that group_ends is not empty");
            }
            let group_end_index = group_start_index + bytes.len();//Exclusive

            self.memory[group_start_index..group_end_index].copy_from_slice(bytes);
            self.group_ends.push(group_end_index);

            Ok(())
        }
    }

    fn get_byte_group(&mut self, group: usize) -> Option<&mut [u8]> {
        if self.num_byte_groups() == 0 {
            None
        } else {
            let group_start_index;//Inclusive
            if group == 0 {
                group_start_index = 0;
            } else {
                group_start_index = self.group_ends[group - 1];
            }
            let group_end_index = self.group_ends[group];//Exclusive

            Some(&mut self.memory[group_start_index..group_end_index])
        }
    }

    //You will likely have to transmute the function pointer to the correct type for your purposes
    //Probably with something like let myfn: extern "C" fn(args) -> ret = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_group()?) };
    fn fn_ptr_to_group(&self, group: usize) -> Option<unsafe fn()> {
        if self.num_byte_groups() == 0 {
            None
        } else {
            let base_memory_ptr = self.memory.as_ptr();
        
            if group == 0 {
                Some(unsafe { std::mem::transmute(base_memory_ptr) })
            } else {
                let offset = self.group_ends[group - 1];
                Some(unsafe { std::mem::transmute(base_memory_ptr.add(offset)) })
            }
        }
    }

    //You will likely have to transmute the function pointer to the correct type for your purposes
    //Probably with something like let myfn: extern "C" fn(args) -> ret = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_start()?) };
    fn fn_ptr_to_start(&self) -> Option<unsafe fn()> {//Equivalent to fn_ptr_to_group(0)
        self.fn_ptr_to_group(0)
    }
}

/* ------------------------------------------------------------------------------------------------
 * Traits And Default Implementations
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Trait Implementations
 * --------------------------------------------------------------------------------------------- */

impl Drop for RWXMemory {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mem_ptr.as_ptr().cast(), self.mem_len);
        }
    }
}

impl AsRef<[u8]> for RWXMemory {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.mem_ptr.as_ptr(), self.mem_len) }
    }
}

impl AsMut<[u8]> for RWXMemory {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.mem_ptr.as_ptr(), self.mem_len) }
    }
}

//RWXMemory is a smart pointer, so it is okay practice to implement Deref and DerefMut
impl std::ops::Deref for RWXMemory {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

//RWXMemory is a smart pointer, so it is okay practice to implement Deref and DerefMut
impl std::ops::DerefMut for RWXMemory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

//TODO add more traits similar to those

/* ------------------------------------------------------------------------------------------------
 * Functions
 * --------------------------------------------------------------------------------------------- */

fn main() {
    println!("Hello, world!");

    let mut jitmemory = JITMemory::new(10000).expect("Failed to allocate memory");

    //mov rax, 0x12345678
    jitmemory.add_byte_group(&[0x48, 0xC7, 0xC0, 0x78, 0x56, 0x34, 0x12]).expect("Failed to add a byte group");
    //ret
    jitmemory.add_byte_group(&[0xC3]).expect("Failed to add a byte group");

    let fn_ptr: extern "C" fn() -> u32 = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_start().unwrap()) };
    println!("fn_ptr() returned 0x{:X}", fn_ptr());

    //Skip the mov. The result will be unpredictable, but it will not crash (we just skip right to the ret)
    let unpredictable_fn_ptr: unsafe extern "C" fn() -> u32 = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_group(1).unwrap()) };
    println!("unpredictable_fn_ptr() returned 0x{:X}", unsafe { unpredictable_fn_ptr() });

    //Add some more instructions to multiple two u64s

    //mov rcx, rsi
    jitmemory.add_byte_group(&[0x48, 0x89, 0xF1]).expect("Failed to add a byte group");
    //mov rax, rdi
    jitmemory.add_byte_group(&[0x48, 0x89, 0xF8]).expect("Failed to add a byte group");
    //mul rcx
    jitmemory.add_byte_group(&[0x48, 0xF7, 0xE1]).expect("Failed to add a byte group");
    //ret
    jitmemory.add_byte_group(&[0xC3]).expect("Failed to add a byte group");

    let multiply: extern "C" fn(u64, u64) -> u64 = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_group(2).unwrap()) };
    println!("3 * 7 = {}", multiply(3, 7));
}

/* ------------------------------------------------------------------------------------------------
 * Tests
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Benchmarks
 * --------------------------------------------------------------------------------------------- */

//TODO
