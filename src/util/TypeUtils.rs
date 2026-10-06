//! Classification and layout operations for C types.

use super::{CheckedLayout, TypeTable, Types::Type};

/// Reject unsupported backend sizes instead of silently truncating them.
pub fn layout_i32(value: i128) -> i32 {
    CheckedLayout::narrow_i32(value)
        .unwrap_or_else(|| panic!("Layout exceeds supported 32-bit size or offset range"))
}
pub fn layout_i64(value: i128) -> i64 {
    CheckedLayout::narrow_i64(value)
        .unwrap_or_else(|| panic!("Layout exceeds supported 64-bit size range"))
}
pub fn get_size_i32(ty: &Type) -> i32 {
    layout_i32(get_size(ty) as i128)
}
pub fn offset_add(base: i32, delta: i64) -> i32 {
    CheckedLayout::offset_add(base, delta)
        .unwrap_or_else(|| panic!("Layout offset exceeds supported 32-bit range"))
}

pub fn get_size(ty: &Type) -> i64 {
    match ty {
        Type::Char | Type::SChar | Type::UChar => 1,
        Type::Int | Type::UInt => 4,
        Type::Long | Type::ULong | Type::Double | Type::Pointer(_) => 8,
        Type::Array(element, size) => {
            CheckedLayout::array_size(layout_i64(*size as i128), get_size(element))
                .unwrap_or_else(|| panic!("Array layout size overflow"))
        }
        Type::Struct(tag) => layout_i64(TypeTable::find(tag).size as i128),
        Type::Function(_, _) | Type::Void => {
            panic!("internal error: type doesn't have size: {ty:?}")
        }
    }
}

pub fn get_alignment(ty: &Type) -> usize {
    match ty {
        Type::Char | Type::SChar | Type::UChar => 1,
        Type::Int | Type::UInt => 4,
        Type::Long | Type::ULong | Type::Double | Type::Pointer(_) => 8,
        Type::Array(element, _) => get_alignment(element),
        Type::Struct(tag) => TypeTable::find(tag).alignment,
        Type::Function(_, _) | Type::Void => {
            panic!("internal error: type doesn't have alignment: {ty:?}")
        }
    }
}

pub fn is_signed(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::Long | Type::Char | Type::SChar => true,
        Type::UInt | Type::ULong | Type::Pointer(_) | Type::UChar => false,
        _ => panic!("internal error: signedness doesn't make sense for non-integral type {ty:?}"),
    }
}
pub fn is_pointer(ty: &Type) -> bool {
    matches!(ty, Type::Pointer(_))
}
pub fn is_integer(ty: &Type) -> bool {
    ty.is_integer()
}
pub fn is_array(ty: &Type) -> bool {
    matches!(ty, Type::Array(_, _))
}
pub fn is_character(ty: &Type) -> bool {
    matches!(ty, Type::Char | Type::SChar | Type::UChar)
}
pub fn is_arithmetic(ty: &Type) -> bool {
    is_integer(ty) || matches!(ty, Type::Double)
}
pub fn is_scalar(ty: &Type) -> bool {
    ty.is_scalar()
}
pub fn is_complete(ty: &Type) -> bool {
    match ty {
        Type::Void => false,
        Type::Struct(tag) => TypeTable::mem(tag),
        _ => true,
    }
}
pub fn is_complete_pointer(ty: &Type) -> bool {
    matches!(ty, Type::Pointer(inner) if is_complete(inner))
}
