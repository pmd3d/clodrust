use vstd::prelude::*;
verus! {
// Historical Rounding.rs implementation: intermediate underflow even though
// the desired final result is representable. This file MUST fail verification.
fn original_rounding(alignment: i64, value: i64) -> (result: i64)
    requires alignment == 8, value == i64::MIN + 1,
    ensures result == i64::MIN,
{
    let remainder = value % alignment;
    if remainder == 0 { value }
    else if value < 0 { value - alignment - remainder }
    else { value + alignment - remainder }
}
fn main() {}
}
