use vstd::prelude::*;
verus! {
// Historical ReplacePseudos.rs subtraction: widening happens too late.
fn original_stack_subtraction(current: i32, size: i32) -> (result: i64)
    requires current == -2147483640, size == 16,
    ensures result as int == current as int - size as int,
{
    (current - size) as i64
}
fn main() {}
}
