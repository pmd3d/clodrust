use clod::ports::{CheckedLayout as L, Rounding, TypeUtils, Types::Type};

#[test]
fn rounding_handles_representable_extremes_and_rejects_overflow() {
    assert_eq!(L::round_away(8, i64::MIN + 1), Some(i64::MIN));
    assert_eq!(Rounding::round_away_from_zero(8, i64::MIN + 1), i64::MIN);
    assert_eq!(L::round_away(8, i64::MAX), None);
    assert_eq!(L::round_up(usize::MAX, 1), Some(usize::MAX));
    assert_eq!(Rounding::round_up(usize::MAX, 1), usize::MAX);
    assert_eq!(L::round_up(usize::MAX, 2), None);
    assert_eq!(L::round_away(0, 9), None);
    assert_eq!(L::round_away(-8, 9), None);
    for value in -64..=64 {
        for alignment in 1..=16 {
            let rounded = L::round_away(alignment, value).unwrap();
            assert_eq!(rounded % alignment, 0);
            assert!(rounded.abs() >= value.abs());
            assert!(rounded.abs() - value.abs() < alignment);
        }
    }
}

#[test]
fn sizes_and_narrowing_never_wrap() {
    assert_eq!(L::array_size(i64::MAX, 1), Some(i64::MAX));
    assert_eq!(L::array_size(i64::MAX, 2), None);
    assert_eq!(L::array_size(-1, 4), None);
    assert_eq!(L::narrow_i32(4_294_967_296), None);
    assert_eq!(L::narrow_i32(i32::MIN as i128), Some(i32::MIN));
    assert_eq!(L::narrow_i64(i64::MAX as i128 + 1), None);
    let giant = Type::Array(
        Box::new(Type::Array(Box::new(Type::Long), i32::MAX as usize)),
        i32::MAX as usize,
    );
    assert!(std::panic::catch_unwind(|| TypeUtils::get_size(&giant)).is_err());
}

#[test]
fn structure_layout_accounts_for_padding_and_nested_sizes() {
    assert_eq!(
        L::struct_layout(&vec![1, 8, 4], &vec![1, 8, 4]),
        Some((vec![0, 8, 16], 24, 8))
    );
    assert_eq!(
        L::struct_layout(&vec![24, 1], &vec![8, 1]),
        Some((vec![0, 24], 32, 8))
    );
    assert_eq!(L::struct_layout(&vec![i32::MAX as i64], &vec![8]), None);
    assert_eq!(L::struct_layout(&vec![1], &vec![0]), None);
    assert_eq!(L::struct_layout(&vec![1], &vec![]), None);
}

#[test]
fn stack_slots_are_aligned_and_disjoint() {
    let mut current = -8; // hidden structure return pointer
    let mut prior = vec![(-8, 8)];
    for (size, alignment) in [(1, 1), (8, 8), (24, 8), (4, 4), (17, 16)] {
        let offset = L::stack_slot(current, size, alignment).unwrap();
        assert_eq!(offset % alignment, 0);
        assert!(offset + size <= current);
        for &(start, len) in &prior {
            assert!(offset + size <= start || start + len <= offset);
        }
        prior.push((offset, size));
        current = offset;
    }
    assert_eq!(L::stack_slot(i32::MIN + 8, 16, 8), None);
    assert_eq!(L::offset_add(i32::MAX, 1), None);
    assert_eq!(L::stack_slot(0, 0, 8), None);
}

#[test]
fn frame_adjustment_accounts_for_saves_and_alignment() {
    for bytes in [0, 1, 8, 17, 128] {
        for saves in 0..=5 {
            let adjust = L::frame_adjustment(bytes, saves).unwrap() as i64;
            assert!(adjust >= bytes);
            assert_eq!((adjust + saves * 8) % 16, 0);
            assert!(adjust - bytes < 16);
        }
    }
    assert_eq!(L::frame_adjustment(i32::MAX as i64, 0), None);
    assert_eq!(L::frame_adjustment(i64::MAX, i64::MAX), None);
    assert_eq!(L::frame_adjustment(-1, 0), None);
}

#[test]
fn large_constant_displacements_require_register_lowering() {
    assert_eq!(L::scaled_offset(1_073_741_824, 4), None);
    assert_eq!(L::scaled_offset(i64::MIN, i32::MIN), None);
    assert_eq!(L::scaled_offset(-3, 8), Some(-24));
}

#[test]
fn compiler_rejects_unsupported_layouts_without_assembly() {
    use clod::ports::{Compile, CompilerError::CompilerError, Settings::*};
    let config = CompilerConfig {
        Debug: false,
        Platform: Target::Linux,
    };
    let options = Optimizations::default();
    for source in [
        "long checked_stride(int (*p)[1073741824]) { return (char *)(p+1)-(char *)p; }",
        "int checked_array[1073741824];",
        "struct checked_struct { char a[2147483647]; char b; };",
    ] {
        let result = Compile::compile(
            &config,
            Stage::Validate,
            &options,
            "checked-layout.c",
            source,
        );
        assert!(
            matches!(result, Err(CompilerError::TypeError(ref msg)) if msg.contains("supported")),
            "{result:?}"
        );
    }
}

#[test]
fn cli_lowers_large_constant_pointer_offsets_without_truncation() {
    // Compilation and GCC linking only; no giant object needs to be allocated.
    let dir = std::env::temp_dir().join(format!("clod-layout-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("offset.c");
    std::fs::write(
        &input,
        "char *checked_offset(char *p) { return p + 4294967296L; }",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_clod--"))
        .arg("-S")
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let assembly = std::fs::read_to_string(input.with_extension("s")).unwrap();
    assert!(assembly.contains("4294967296"), "{assembly}");
    let output = std::process::Command::new("gcc")
        .arg("-c")
        .arg(input.with_extension("s"))
        .arg("-o")
        .arg(dir.join("offset.o"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cli_executes_nested_struct_and_array_layout() {
    let dir = std::env::temp_dir().join(format!("clod-struct-layout-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("layout.c");
    std::fs::write(
        &input,
        r#"
        struct Inner { char c; long n; };
        struct Outer { char tag; struct Inner inner; int xs[3]; };
        int main(void) {
            struct Outer x = {65, {66, 123L}, {4, 5, 6}};
            if (sizeof(struct Inner) != 16UL) return 1;
            if (sizeof(struct Outer) != 40UL) return 2;
            if ((char *)&x.inner - (char *)&x != 8L) return 3;
            if ((char *)&x.inner.n - (char *)&x != 16L) return 4;
            if ((char *)&x.xs - (char *)&x != 24L) return 5;
            if (x.inner.n != 123L) return 6;
            if (x.xs[2] != 6) return 7;
            return 0;
        }
    "#,
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_clod--"))
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::process::Command::new(input.with_extension(""))
        .status()
        .unwrap()
        .success());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cli_rejects_unrepresentable_aligned_stack_frame() {
    let dir = std::env::temp_dir().join(format!("clod-stack-layout-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("stack.c");
    std::fs::write(
        &input,
        "int main(void) { char a[2147483647]; return a[0]; }",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_clod--"))
        .arg("-S")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("supported"));
    assert!(!input.with_extension("s").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
