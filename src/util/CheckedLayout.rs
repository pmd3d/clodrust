//! Checked arithmetic shared by the compiler and Verus.
//!
//! `verification/verify.py` activates only `// verus:` annotations. Executable
//! bodies below are compiled unchanged by Cargo and verified by Verus.

// verus: pub open spec fn fits_i64(v: int) -> bool { i64::MIN <= v <= i64::MAX }
// verus: pub open spec fn fits_i32(v: int) -> bool { i32::MIN <= v <= i32::MAX }
// verus: pub open spec fn fits_usize(v: int) -> bool { 0 <= v <= usize::MAX }
// verus: pub open spec fn rounded(a: int, v: int) -> int {
// verus:     if a <= 0 { 0 } else if v < 0 { -(((-v + a - 1) / a) * a) }
// verus:     else { ((v + a - 1) / a) * a }
// verus: }

pub fn narrow_i64(value: i128) -> Option<i64>
// verus: ensures match result { Some(v) => v as int == value as int, None => !fits_i64(value as int) },
{
    if value < i64::MIN as i128 || value > i64::MAX as i128 {
        None
    } else {
        Some(value as i64)
    }
}

pub fn narrow_i32(value: i128) -> Option<i32>
// verus: ensures match result { Some(v) => v as int == value as int, None => !fits_i32(value as int) },
{
    if value < i32::MIN as i128 || value > i32::MAX as i128 {
        None
    } else {
        Some(value as i32)
    }
}

pub fn narrow_usize(value: i128) -> Option<usize>
// verus: ensures match result { Some(v) => v as int == value as int, None => !fits_usize(value as int) },
{
    if value < 0 || value > usize::MAX as i128 {
        None
    } else {
        Some(value as usize)
    }
}

pub fn round_up(value: usize, alignment: usize) -> Option<usize>
// verus: ensures match result {
// verus:     Some(v) => alignment > 0 && v as int == rounded(alignment as int, value as int),
// verus:     None => alignment == 0 || !fits_usize(rounded(alignment as int, value as int)),
// verus: },
{
    if alignment == 0 {
        return None;
    }
    let a = alignment as i128;
    let numerator = value as i128 + a - 1;
    let quotient = numerator / a;
    // verus: proof {
    // verus:     vstd::arithmetic::div_mod::lemma_fundamental_div_mod(numerator as int, a as int);
    // verus:     vstd::arithmetic::div_mod::lemma_mod_bound(numerator as int, a as int);
    // verus:     vstd::arithmetic::div_mod::lemma_div_is_ordered(0, numerator as int, a as int);
    // verus:     assert(0 <= quotient as int * a as int <= numerator as int) by (nonlinear_arith)
    // verus:         requires quotient >= 0, a > 0, numerator as int == a as int * quotient as int + numerator as int % a as int,
    // verus:             numerator as int % a as int >= 0;
    // verus: }
    narrow_usize(quotient * a)
}

/// Negative sizes/counts are invalid. Zero counts are supported by the kernel.
pub fn array_size(count: i64, element_size: i64) -> Option<i64>
// verus: ensures match result {
// verus:     Some(v) => count >= 0 && element_size >= 0 && v as int == count as int * element_size as int,
// verus:     None => count < 0 || element_size < 0 || !fits_i64(count as int * element_size as int),
// verus: },
{
    if count < 0 || element_size < 0 {
        return None;
    }
    // verus: assert(0 <= count as int * element_size as int <= i128::MAX) by (nonlinear_arith)
    // verus:     requires 0 <= count <= i64::MAX, 0 <= element_size <= i64::MAX;
    narrow_i64(count as i128 * element_size as i128)
}

/// Round without taking abs(i64::MIN) or overflowing an intermediate.
pub fn round_away(alignment: i64, value: i64) -> Option<i64>
// verus: ensures match result {
// verus:     Some(v) => alignment > 0 && v as int == rounded(alignment as int, value as int),
// verus:     None => alignment <= 0 || !fits_i64(rounded(alignment as int, value as int)),
// verus: },
{
    if alignment <= 0 {
        return None;
    }
    let magnitude = if value < 0 {
        -(value as i128)
    } else {
        value as i128
    };
    let a = alignment as i128;
    let numerator = magnitude + a - 1;
    let quotient = numerator / a;
    // verus: assert(0 <= quotient <= numerator) by {
    // verus:     vstd::arithmetic::div_mod::lemma_div_is_ordered(0, numerator as int, a as int);
    // verus:     vstd::arithmetic::div_mod::lemma_div_nonincreasing(numerator as int, a as int);
    // verus: }
    // verus: assert(0 <= quotient as int * a as int <= i128::MAX) by (nonlinear_arith)
    // verus:     requires 0 <= quotient <= 18446744073709551614int, 0 < a <= i64::MAX;
    let result = quotient * a;
    narrow_i64(if value < 0 { -result } else { result })
}

/// Append a member to a structure prefix; returns its offset and the new end.
pub fn member_layout(current: i64, size: i64, alignment: i64) -> Option<(i64, i64)>
// verus: ensures match result {
// verus:     Some((offset, end)) => current >= 0 && size >= 0 && alignment > 0
// verus:         && offset as int == rounded(alignment as int, current as int)
// verus:         && end as int == offset as int + size as int && offset >= current,
// verus:     None => current < 0 || size < 0 || alignment <= 0
// verus:         || !fits_i64(rounded(alignment as int, current as int))
// verus:         || !fits_i64(rounded(alignment as int, current as int) + size as int),
// verus: },
{
    if current < 0 || size < 0 {
        return None;
    }
    let offset = match round_away(alignment, current) {
        Some(v) => v,
        None => return None,
    };
    // verus: proof { round_properties(alignment as int, current as int); }
    match narrow_i64(offset as i128 + size as i128) {
        Some(end) => Some((offset, end)),
        None => None,
    }
}

/// Allocate below a nonpositive frame offset. Existing slots end at/above current.
pub fn stack_slot(current: i32, size: i32, alignment: i32) -> Option<i32>
// verus: ensures match result {
// verus:     Some(offset) => current <= 0 && size > 0 && alignment > 0
// verus:         && offset as int == rounded(alignment as int, current as int - size as int)
// verus:         && offset as int + size as int <= current as int && offset < current,
// verus:     None => current > 0 || size <= 0 || alignment <= 0
// verus:         || !fits_i32(rounded(alignment as int, current as int - size as int)),
// verus: },
{
    if current > 0 || size <= 0 || alignment <= 0 {
        return None;
    }
    let unaligned = current as i64 - size as i64;
    // verus: proof { round_properties(alignment as int, unaligned as int); }
    match round_away(alignment as i64, unaligned) {
        Some(offset) => narrow_i32(offset as i128),
        None => None,
    }
}

pub fn offset_add(base: i32, delta: i64) -> Option<i32>
// verus: ensures match result {
// verus:     Some(v) => v as int == base as int + delta as int,
// verus:     None => !fits_i32(base as int + delta as int),
// verus: },
{
    narrow_i32(base as i128 + delta as i128)
}

/// A constant pointer displacement is usable directly only if it fits x86 disp32.
pub fn scaled_offset(index: i64, scale: i32) -> Option<i32>
// verus: ensures match result {
// verus:     Some(v) => v as int == index as int * scale as int,
// verus:     None => !fits_i32(index as int * scale as int),
// verus: },
{
    // verus: assert(i128::MIN <= index as int * scale as int <= i128::MAX) by (nonlinear_arith)
    // verus:     requires i64::MIN <= index <= i64::MAX, i32::MIN <= scale <= i32::MAX;
    narrow_i32(index as i128 * scale as i128)
}

/// Return the subtraction before saved-register pushes; total frame is disp32-sized.
pub fn frame_adjustment(bytes: i64, saved_count: i64) -> Option<i32>
// verus: ensures match result {
// verus:     Some(v) => bytes >= 0 && saved_count >= 0 && v >= bytes
// verus:         && v as int + saved_count as int * 8 == rounded(16, bytes as int + saved_count as int * 8)
// verus:         && v as int + saved_count as int * 8 <= i32::MAX,
// verus:     None => bytes < 0 || saved_count < 0
// verus:         || rounded(16, bytes as int + saved_count as int * 8) > i32::MAX,
// verus: },
{
    if bytes < 0 || saved_count < 0 {
        return None;
    }
    let saved = saved_count as i128 * 8;
    let total = bytes as i128 + saved;
    // Reject before narrowing, including alignment padding and saved registers.
    if total > i32::MAX as i128 {
        // verus: proof { round_properties(16, total as int); }
        return None;
    }
    let aligned = match round_away(16, total as i64) {
        Some(v) => v,
        None => return None,
    };
    // verus: proof { round_properties(16, total as int); }
    if aligned > i32::MAX as i64 {
        return None;
    }
    narrow_i32(aligned as i128 - saved)
}

/// Lay out members in declaration order, including final structure padding.
pub fn struct_layout(sizes: &Vec<i64>, alignments: &Vec<i64>) -> Option<(Vec<i64>, i64, i64)>
// verus: ensures match result {
// verus:     Some((offsets, size, alignment)) => offsets.len() == sizes.len() && sizes.len() == alignments.len()
// verus:         && 0 <= size <= i32::MAX && alignment > 0 && size as int % alignment as int == 0
// verus:         && (alignment == 1 || exists|j: int| 0 <= j < alignments.len() && alignments[j] == alignment)
// verus:         && size as int == rounded(alignment as int, if offsets.len() == 0 { 0 }
// verus:             else { offsets[offsets.len() as int-1] as int + sizes[sizes.len() as int-1] as int })
// verus:         && (forall|j: int| 0 <= j < sizes.len() ==> sizes[j] >= 0 && alignments[j] > 0
// verus:             && 0 <= offsets[j] && offsets[j] as int + sizes[j] as int <= size as int
// verus:             && offsets[j] as int % alignments[j] as int == 0 && alignment >= alignments[j])
// verus:         && (forall|j: int, k: int| 0 <= j < k < sizes.len() ==> offsets[j] as int + sizes[j] as int <= offsets[k] as int)
// verus:         && (forall|j: int| 0 <= j < sizes.len() ==> offsets[j] as int == rounded(alignments[j] as int,
// verus:             if j == 0 { 0 } else { offsets[j-1] as int + sizes[j-1] as int })),
// verus:     None => true,
// verus: },
{
    if sizes.len() != alignments.len() {
        return None;
    }
    let mut offsets = Vec::new();
    let mut end = 0i64;
    let mut alignment = 1i64;
    let mut i = 0usize;
    while i < sizes.len()
    // verus: invariant
    // verus:     i <= sizes.len(), sizes.len() == alignments.len(), offsets.len() == i,
    // verus:     0 <= end <= i32::MAX, alignment > 0,
    // verus:     alignment == 1 || exists|j: int| 0 <= j < i && alignments[j] == alignment,
    // verus:     i == 0 ==> end == 0,
    // verus:     i > 0 ==> end as int == offsets[i as int-1] as int + sizes[i as int-1] as int,
    // verus:     forall|j: int| 0 <= j < i ==> sizes[j] >= 0 && alignments[j] > 0
    // verus:         && 0 <= offsets[j] && offsets[j] as int + sizes[j] as int <= end as int
    // verus:         && offsets[j] as int % alignments[j] as int == 0 && alignment >= alignments[j],
    // verus:     forall|j: int, k: int| 0 <= j < k < i ==> offsets[j] as int + sizes[j] as int <= offsets[k] as int,
    // verus:     forall|j: int| 0 <= j < i ==> offsets[j] as int == rounded(alignments[j] as int,
    // verus:         if j == 0 { 0 } else { offsets[j-1] as int + sizes[j-1] as int }),
    // verus: decreases sizes.len() - i,
    {
        let size = sizes[i];
        let a = alignments[i];
        let (offset, next_end) = match member_layout(end, size, a) {
            Some(pair) => pair,
            None => return None,
        };
        if next_end > i32::MAX as i64 {
            return None;
        }
        // verus: proof { round_properties(a as int, end as int); }
        offsets.push(offset);
        // verus: assert(offsets[i as int] == offset);
        if a > alignment {
            alignment = a;
            // verus: assert(alignments[i as int] == alignment);
        }
        // verus: assert forall|j: int| 0 <= j < i + 1 implies sizes[j] >= 0 && alignments[j] > 0
        // verus:     && 0 <= offsets[j] && offsets[j] as int + sizes[j] as int <= next_end as int
        // verus:     && offsets[j] as int % alignments[j] as int == 0 && alignment >= alignments[j] by {
        // verus:     if j < i { assert(offsets[j] as int + sizes[j] as int <= end as int); }
        // verus:     else { assert(j == i); }
        // verus: }
        // verus: assert forall|j: int, k: int| 0 <= j < k < i + 1 implies offsets[j] as int + sizes[j] as int <= offsets[k] as int by {
        // verus:     if k < i { assert(offsets[j] as int + sizes[j] as int <= offsets[k] as int); }
        // verus:     else { assert(k == i); assert(offsets[j] as int + sizes[j] as int <= end as int); }
        // verus: }
        // verus: assert forall|j: int| 0 <= j < i + 1 implies offsets[j] as int == rounded(alignments[j] as int,
        // verus:     if j == 0 { 0 } else { offsets[j-1] as int + sizes[j-1] as int }) by {
        // verus:     if j < i { assert(offsets[j] as int == rounded(alignments[j] as int,
        // verus:         if j == 0 { 0 } else { offsets[j-1] as int + sizes[j-1] as int })); }
        // verus:     else { assert(j == i); }
        // verus: }
        end = next_end;
        i += 1;
    }
    let size = match round_away(alignment, end) {
        Some(v) => v,
        None => return None,
    };
    if size > i32::MAX as i64 {
        return None;
    }
    // verus: proof { round_properties(alignment as int, end as int); }
    Some((offsets, size, alignment))
}
