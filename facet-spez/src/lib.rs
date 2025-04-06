#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
pub use ::impls::impls;
use core::fmt::{self, Debug};
use facet_types::ParseError;

use facet_opaque::{Opaque, OpaqueUninit};

/// A wrapper type used for auto-deref specialization.
///
/// This struct is a core part of the auto-deref-based specialization technique which allows
/// conditionally implementing functionality based on what traits a type implements, without
/// requiring the unstable `specialization` feature.
///
/// It wraps a value and is used in conjunction with trait implementations that leverage
/// Rust's method resolution rules to select different implementations based on available traits.
pub struct Spez<T>(pub T);

//////////////////////////////////////////////////////////////////////////////////////
// Debug 🐛🔍
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::fmt::Debug`]
pub trait SpezDebugYes {
    /// Delegates to the inner type's `Debug` implementation.
    ///
    /// This method is called when the wrapped type implements `Debug`.
    /// It forwards the formatting request to the inner value's `Debug` implementation.
    fn spez_debug(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error>;
}
impl<T: Debug> SpezDebugYes for &Spez<T> {
    fn spez_debug(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        Debug::fmt(&self.0, f)
    }
}

/// Specialization proxy for [`std::fmt::Debug`]
pub trait SpezDebugNo {
    /// Fallback implementation when the type doesn't implement `Debug`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `Debug`.
    fn spez_debug(&self, _f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error>;
}
impl<T> SpezDebugNo for Spez<T> {
    fn spez_debug(&self, _f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        unreachable!()
    }
}

//////////////////////////////////////////////////////////////////////////////////////
// Display 📺🖥️
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::fmt::Display`]
pub trait SpezDisplayYes {
    /// Delegates to the inner type's `Display` implementation.
    ///
    /// This method is called when the wrapped type implements `Display`.
    /// It forwards the formatting request to the inner value's `Display` implementation.
    fn spez_display(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error>;
}
impl<T: fmt::Display> SpezDisplayYes for &Spez<T> {
    fn spez_display(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Specialization proxy for [`std::fmt::Display`]
pub trait SpezDisplayNo {
    /// Fallback implementation when the type doesn't implement `Display`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `Display`.
    fn spez_display(&self, _f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error>;
}
impl<T> SpezDisplayNo for Spez<T> {
    fn spez_display(&self, _f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        unreachable!()
    }
}

//////////////////////////////////////////////////////////////////////////////////////
// Default (in place, because we can't have sized) 🏠🔄
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::default::Default`]
pub trait SpezDefaultInPlaceYes {
    /// Creates a default value for the inner type in place.
    ///
    /// This method is called when the wrapped type implements `Default`.
    /// It writes the default value into the provided uninitialized memory.
    fn spez_default_in_place<'mem>(&self, target: OpaqueUninit<'mem>) -> Opaque<'mem>;
}
impl<T: Default> SpezDefaultInPlaceYes for &Spez<T> {
    fn spez_default_in_place<'mem>(&self, target: OpaqueUninit<'mem>) -> Opaque<'mem> {
        unsafe { target.write(<T as Default>::default()) }
    }
}

/// Specialization proxy for [`std::default::Default`]
pub trait SpezDefaultInPlaceNo {
    /// Fallback implementation when the type doesn't implement `Default`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `Default`.
    fn spez_default_in_place<'mem>(&self, _target: OpaqueUninit<'mem>) -> Opaque<'mem>;
}
impl<T> SpezDefaultInPlaceNo for Spez<T> {
    fn spez_default_in_place<'mem>(&self, _target: OpaqueUninit<'mem>) -> Opaque<'mem> {
        unreachable!()
    }
}

//////////////////////////////////////////////////////////////////////////////////////
// Clone into 🐑📥
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::clone::Clone`]
pub trait SpezCloneIntoYes {
    /// Clones the inner value into the provided uninitialized memory.
    ///
    /// This method is called when the wrapped type implements `Clone`.
    /// It creates a clone of the inner value and writes it into the target memory.
    fn spez_clone_into<'mem>(&self, target: OpaqueUninit<'mem>) -> Opaque<'mem>;
}
impl<T: Clone> SpezCloneIntoYes for &Spez<T> {
    fn spez_clone_into<'mem>(&self, target: OpaqueUninit<'mem>) -> Opaque<'mem> {
        unsafe { target.write(self.0.clone()) }
    }
}

/// Specialization proxy for [`std::clone::Clone`]
pub trait SpezCloneIntoNo {
    /// Fallback implementation when the type doesn't implement `Clone`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `Clone`.
    fn spez_clone_into<'mem>(&self, _target: OpaqueUninit<'mem>) -> Opaque<'mem>;
}
impl<T> SpezCloneIntoNo for Spez<T> {
    fn spez_clone_into<'mem>(&self, _target: OpaqueUninit<'mem>) -> Opaque<'mem> {
        unreachable!()
    }
}

//////////////////////////////////////////////////////////////////////////////////////
// Parse 📝🔍
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::str::FromStr`]
pub trait SpezParseYes {
    /// Parses a string slice into the inner type.
    ///
    /// This method is called when the wrapped type implements `FromStr`.
    /// It attempts to parse the provided string and write the result into the target memory.
    fn spez_parse(&self, s: &str, target: OpaqueUninit) -> Result<(), ParseError>;
}
impl<T: core::str::FromStr> SpezParseYes for &Spez<T> {
    fn spez_parse(&self, s: &str, target: OpaqueUninit) -> Result<(), ParseError> {
        match <T as core::str::FromStr>::from_str(s) {
            Ok(value) => {
                unsafe { target.write(value) };
                Ok(())
            }
            Err(_) => Err(ParseError::Generic(
                const { concat!("parse error for ", stringify!(T)) },
            )),
        }
    }
}

/// Specialization proxy for [`std::str::FromStr`]
pub trait SpezParseNo {
    /// Fallback implementation when the type doesn't implement `FromStr`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `FromStr`.
    fn spez_parse(&self, _s: &str, _target: OpaqueUninit) -> Result<(), ParseError>;
}
impl<T> SpezParseNo for Spez<T> {
    fn spez_parse(&self, _s: &str, _target: OpaqueUninit) -> Result<(), ParseError> {
        unreachable!()
    }
}

//////////////////////////////////////////////////////////////////////////////////////
// PartialEq 🟰🤝
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::cmp::PartialEq`]
pub trait SpezPartialEqYes {
    /// Checks if two values are equal.
    ///
    /// This method is called when the wrapped type implements `PartialEq`.
    /// It compares the inner values for equality.
    fn spez_eq(&self, other: &Self) -> bool;
}
impl<T: PartialEq> SpezPartialEqYes for &Spez<T> {
    fn spez_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// Specialization proxy for [`std::cmp::PartialEq`]
pub trait SpezPartialEqNo {
    /// Fallback implementation when the type doesn't implement `PartialEq`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `PartialEq`.
    fn spez_eq(&self, _other: &Self) -> bool;
}
impl<T> SpezPartialEqNo for Spez<T> {
    fn spez_eq(&self, _other: &Self) -> bool {
        unreachable!()
    }
}

//////////////////////////////////////////////////////////////////////////////////////
// PartialOrd 🔢↕️
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::cmp::PartialOrd`]
pub trait SpezPartialOrdYes {
    /// Compares two values for ordering.
    ///
    /// This method is called when the wrapped type implements `PartialOrd`.
    /// It compares the inner values and returns their ordering.
    fn spez_partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>;
}
impl<T: PartialOrd> SpezPartialOrdYes for &Spez<T> {
    fn spez_partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

/// Specialization proxy for [`std::cmp::PartialOrd`]
pub trait SpezPartialOrdNo {
    /// Fallback implementation when the type doesn't implement `PartialOrd`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `PartialOrd`.
    fn spez_partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering>;
}
impl<T> SpezPartialOrdNo for Spez<T> {
    fn spez_partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        unreachable!()
    }
}

//////////////////////////////////////////////////////////////////////////////////////
// Ord 📊🔀
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::cmp::Ord`]
pub trait SpezOrdYes {
    /// Compares two values for ordering.
    ///
    /// This method is called when the wrapped type implements `Ord`.
    /// It compares the inner values and returns their ordering.
    fn spez_cmp(&self, other: &Self) -> std::cmp::Ordering;
}
impl<T: Ord> SpezOrdYes for &Spez<T> {
    fn spez_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

/// Specialization proxy for [`std::cmp::Ord`]
pub trait SpezOrdNo {
    /// Fallback implementation when the type doesn't implement `Ord`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `Ord`.
    fn spez_cmp(&self, _other: &Self) -> std::cmp::Ordering;
}
impl<T> SpezOrdNo for Spez<T> {
    fn spez_cmp(&self, _other: &Self) -> std::cmp::Ordering {
        unreachable!()
    }
}

//////////////////////////////////////////////////////////////////////////////////////
// Hash #️⃣🔐
//////////////////////////////////////////////////////////////////////////////////////

/// Specialization proxy for [`std::hash::Hash`]
pub trait SpezHashYes {
    /// Hashes the inner value.
    ///
    /// This method is called when the wrapped type implements `Hash`.
    /// It hashes the inner value using the provided hasher.
    fn spez_hash<H: std::hash::Hasher>(&self, state: &mut H);
}
impl<T: std::hash::Hash> SpezHashYes for &Spez<T> {
    fn spez_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

/// Specialization proxy for [`std::hash::Hash`]
pub trait SpezHashNo {
    /// Fallback implementation when the type doesn't implement `Hash`.
    ///
    /// This method is used as a fallback and is designed to be unreachable in practice.
    /// It's only selected when the wrapped type doesn't implement `Hash`.
    fn spez_hash<H: std::hash::Hasher>(&self, _state: &mut H);
}
impl<T> SpezHashNo for Spez<T> {
    fn spez_hash<H: std::hash::Hasher>(&self, _state: &mut H) {
        unreachable!()
    }
}
