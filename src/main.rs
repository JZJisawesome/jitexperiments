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
            unsafe { std::ptr::write_bytes(mem_ptr, 0, size_bytes); }

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
    fn take_ptr(self) -> std::ptr::NonNull<u8> {
        let ptr = self.mem_ptr;
        std::mem::forget(self);
        ptr
    }
}

impl JITMemory {
    fn new(size_bytes: usize) -> Option<Self> {
        let mut memory = RWXMemory::new(size_bytes)?;

        //TODO do this differently for other architectures
        //Fill the page with a nop slide into an illegal opcode to help catch bugs
        //IT IS EXPECTED THAT THE BYTES THAT ARE WRITTEN RETURN values that make sense for you
        memory.fill(0x90);//nop
        let len = memory.len();
        debug_assert!(len >= 2);
        memory[len - 2] = 0x0F;//Start of ud
        memory[len - 1] = 0xFF;//End of ud
        //End of amd64 specific code

        Some(
            JITMemory {
                memory,
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
            let group_start_index;
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

    //You will likely have to transmute the function pointer to the correct type for your purposes
    //Probably with something like let myfn: extern "C" fn(args) -> ret = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_group()) };
    fn fn_ptr_to_group(&self, group: usize) -> unsafe fn() {
        let mut base_memory_ptr = self.memory.as_ptr();
    
        if group == 0 {
            unsafe { std::mem::transmute(base_memory_ptr) }
        } else {
            let offset = self.group_ends[group - 1];
            unsafe { std::mem::transmute(base_memory_ptr.add(offset)) }
        }
    }

    //You will likely have to transmute the function pointer to the correct type for your purposes
    //Probably with something like let myfn: extern "C" fn(args) -> ret = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_start()) };
    fn fn_ptr_to_start(&self) -> unsafe fn() {//Equivalent to fn_ptr_to_group(0), but also works when there are no groups
        unsafe { std::mem::transmute(self.memory.as_ptr()) }
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

    let mut jitmemory = JITMemory::new(4096).expect("Failed to allocate a page of memory");

    //mov rax, 0x12345678
    jitmemory.add_byte_group(&[0x48, 0xC7, 0xC0, 0x78, 0x56, 0x34, 0x12]).expect("Failed to add a byte group");
    //ret
    jitmemory.add_byte_group(&[0xC3]).expect("Failed to add a byte group");

    let fn_ptr: extern "C" fn() -> u32 = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_start()) };
    println!("fn_ptr() returned 0x{:X}", fn_ptr());

    //Skip the mov. The result will be unpredictable, but it will not crash (we just skip right to the ret)
    let unpredictable_fn_ptr: unsafe extern "C" fn() -> u32 = unsafe { std::mem::transmute(jitmemory.fn_ptr_to_group(1)) };
    println!("unpredictable_fn_ptr() returned 0x{:X}", unsafe { unpredictable_fn_ptr() });

    //This will cause an illegal instruction exception
    /*unsafe {
        jitmemory.fn_ptr_to_start()();
    }*/
}

/* ------------------------------------------------------------------------------------------------
 * Tests
 * --------------------------------------------------------------------------------------------- */

//TODO

/* ------------------------------------------------------------------------------------------------
 * Benchmarks
 * --------------------------------------------------------------------------------------------- */

//TODO
