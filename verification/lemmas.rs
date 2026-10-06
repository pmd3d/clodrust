// Specification lemmas only: production execution uses no trusted proof bodies.
proof fn round_properties(a: int, value: int)
    requires a > 0,
    ensures
        rounded(a, value) % a == 0,
        value >= 0 ==> rounded(a, value) >= value && rounded(a, value) - value < a,
        value < 0 ==> rounded(a, value) <= value && value - rounded(a, value) < a,
{
    reveal(rounded);
    let magnitude = if value < 0 { -value } else { value };
    let n = magnitude + a - 1;
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(n, a);
    vstd::arithmetic::div_mod::lemma_mod_bound(n, a);
    assert((n / a) * a == n - n % a) by (nonlinear_arith)
        requires n == a * (n / a) + n % a;
    vstd::arithmetic::div_mod::lemma_mod_multiples_basic(n / a, a);
    vstd::arithmetic::div_mod::lemma_mod_multiples_basic(-(n / a), a);
    assert(-((n / a) * a) == (-(n / a)) * a) by (nonlinear_arith);
    if value < 0 {
        assert(rounded(a, value) == (-(n / a)) * a);
    } else {
        assert(rounded(a, value) == (n / a) * a);
    }
}

proof fn earlier_slot_disjoint(current: int, old_start: int, old_size: int, new_start: int, new_size: int)
    requires old_size >= 0, new_size > 0, old_start >= current, new_start + new_size <= current,
    ensures new_start + new_size <= old_start,
{}
