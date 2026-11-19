use std::alloc::System;

use cap::Cap;

const CEILING: usize = 1_024 * 1_024 * 1_024;

#[global_allocator]
static ALLOCATOR: Cap<System> = Cap::new(System, CEILING);
