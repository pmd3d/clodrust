# Checked arithmetic and layout with Verus

This milestone verifies the executable kernels used for size arithmetic,
structure placement, pointer displacement, and stack/frame layout. It also
includes known failing examples and mutations so you can check that the proofs
detect bugs.

## Run it

From the repository root on x86_64 Linux or WSL:

```sh
./verification/install.sh
python3 verification/verify.py
python3 verification/verify.py --bugs
python3 verification/verify.py --mutations
cargo test
```

Prerequisites are Rust/rustup, Python 3, curl, unzip, and sha256sum. Compiler
integration tests also use GCC. Installation downloads the pinned Verus release
into the ignored `.verus/` directory and installs its Rust toolchain alongside
your existing one. It does not change your default Rust toolchain. The download
is about 463 MiB and expands to about 1.6 GiB.

`toolchain.json` pins Verus `0.2026.10.04.426d8b0`, Rust `1.98.1`, and the Linux
archive's SHA-256. The runner checks the Verus version. To use an existing
matching installation:

```sh
python3 verification/verify.py --verus /path/to/verus
# Alternatively: export VERUS=/path/to/verus
```

On macOS, download the matching platform archive from the
[pinned release](https://github.com/verus-lang/verus/releases/tag/release/0.2026.10.04.426d8b0),
install its required toolchain with rustup, and supply `--verus`. Automatic
installation and the compiler integration tests were exercised on Linux.
The managed workspace fallback finds Rust under the checkout's parent's
`.cargo/` and `.rustup/` directories if the normal home installation is absent.

## Expected results

The normal command currently reports **22 verified, 0 errors**. A nonzero exit
means the actual production kernels no longer satisfy their contracts, the
verifier needs another proof step, or the verification setup has an error.

`--bugs` runs three historical compiler implementations and requires Verus to
reject each:

| Fixture | Bug | Expected diagnostic |
| --- | --- | --- |
| `bugs/stride.rs` | 4 GiB pointee size truncated to a zero `i32` stride | Failed postcondition |
| `bugs/rounding.rs` | Signed rounding underflows even though its final result fits | Arithmetic underflow/overflow |
| `bugs/stack.rs` | Stack subtraction overflows before widening | Arithmetic underflow/overflow |

This mode exits zero only when all three expected verification failures occur.
To see an individual failing command return nonzero, run Verus directly:

```sh
.verus/verus-x86-linux/verus verification/bugs/stride.rs
```

`--mutations` first verifies the unchanged production kernels, then generates
seven deliberately broken versions under `target/verification/mutations/`.
It changes narrowing, array size, rounding, stack placement, structure offsets,
saved-register accounting, and pointer displacement. Each must fail a proof.
Source files in `src/` are never changed by this mode. Syntax/type errors and
solver resource exhaustion do **not** count as successful bug detection.

## Find bugs in your changes

Edit `src/util/CheckedLayout.rs`, preserve its contracts, and rerun:

```sh
python3 verification/verify.py
```

Unlike tests of selected inputs, this checks the stated contracts for every
input in the functions' machine-integer domains. Executable functions accept
arbitrary inputs and use `Option` to reject invalid or unrepresentable results;
they do not assume small sizes through caller preconditions.

Contracts are comments beginning `// verus:` beside the production function.
They specify exact mathematical results, checked rejection, loop invariants,
and proof steps. `verify.py` activates those comments, names return values for
the contracts, and wraps the module in `verus!`. **It does not replace or rewrite
the executable function bodies.** Cargo compiles those same bodies as ordinary
Rust without a Verus dependency.

To inspect the exact verifier input:

```sh
python3 verification/verify.py --emit-only
```

The generated file is `target/verification/checked_layout.rs`. Do not edit it;
the next run regenerates it. Diagnostics point to this generated file. Look for
the corresponding function and `// verus:` contract in the production source.
If you change a function's signature, also update its contract and any mutation
fragment affected by that change.

For a new arithmetic operation, add its executable kernel and mathematical
contract there, then call it from production. Code elsewhere in the compiler
is not automatically covered by this runner. Do not weaken a failed contract
or add `assume`/`admit` merely to make verification pass. First determine whether
the failure is a bug, a missing proof step, or a specification mistake. Verus
does not promise a concrete counterexample for every failure; minimize the
case and add a Rust or C regression when you find one.

## Verified properties and production connections

| Kernel | Property | Production use |
| --- | --- | --- |
| `narrow_i32`, `narrow_i64`, `narrow_usize` | Exact value or rejection outside the target range | Type sizes, offsets, zero-fill sizes, frame sizes |
| `array_size` | Exact nonnegative count × element size, or rejection | Recursive array layout and initializer padding |
| `round_away`, `round_up` | Exact rounding and safe intermediate arithmetic, including extrema | Alignment helpers, member placement, stack slots |
| `member_layout` | Aligned offset at/after prefix end; exact member end | Structure kernel |
| `struct_layout` | Declaration-order offsets; aligned, bounded, pairwise nonoverlapping members; maximum alignment and exact final padding | `Typecheck::typecheckStructDecl` |
| `stack_slot` | Exact aligned placement below the previous frame offset | `ReplacePseudos::calculateOffset` |
| `offset_add` | Exact representable offset sum | Aggregate initialization and pseudo/assembly memory offsets |
| `scaled_offset` | Exact displacement when it fits x86 disp32; rejection otherwise | Constant pointer arithmetic fast path |
| `frame_adjustment` | Enough space for locals plus saves; 16-byte aligned total; checked padding | `InstructionFixup::emitStackAdjustment` |

`lemmas.rs` proves rounding divisibility/distance and the numeric implication
that a newly allocated slot is disjoint from earlier slots above the previous
frame offset. There are no `assume`, `admit`, or `external_body` declarations in
this verification target. The proof boundary trusts Verus, its standard library,
Rust's integer/collection model, and the small source-activation script.

The proofs cover the kernels and the full structure-placement loop. They do
not cover AST-to-size metadata extraction, global symbol/type tables, the
stack offset map or its reuse logic, panic-to-diagnostic plumbing, generated
x86 instruction semantics, or the entire compiler. Those connections have
regression tests, including executing a nested structure/array program.
Structure success properties are verified; completeness of structure rejection
is not: returning `None` for an otherwise valid structure would require tests
or a stronger completeness contract to detect.

## Resulting compiler behavior

Array and structure layouts exceeding the backend's signed 32-bit size range
are rejected during validation. Pointer scales, member offsets, zero-fill sizes,
and stack/frame arithmetic use checked conversion or checked layout kernels.
Overflow follows the existing compiler's `TypeError` diagnostic path.

Large pointer **indices** remain supported: if an otherwise valid constant
displacement does not fit disp32, code generation uses a full-width index in a
register instead of truncating the constant. This differs from an unsupported
oversized pointee type, which produces a diagnostic.

`tests/TestCheckedLayout.rs` covers integer extrema, nested size overflow,
structure padding, nonoverlapping stack slots, frame saves/alignment, the large
stride rejection, large constant displacement assembly/GCC assembly, rejection
of an unrepresentable aligned stack frame, and execution of nested layouts.
