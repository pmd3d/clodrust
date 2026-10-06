use vstd::prelude::*;
verus! {
// Historical TackyGen.rs pointer-scale cast: 2^32 becomes zero.
fn original_stride(size: i64) -> (result: i32)
    requires size == 4294967296,
    ensures result as int == size as int,
{
    size as i32
}
fn main() {}
}
