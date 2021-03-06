#![deny(missing_docs)]
#![deny(warnings)]
#![feature(macro_rules)]

//! A generic, extendable Error type.

extern crate typeable;

use std::fmt::{mod, Show, Formatter};
use std::{raw, mem};
use std::intrinsics::TypeId;
use std::error::Error as StdError;
use std::error::FromError;

use typeable::Typeable;

/// An extension to std::error::Error which provides dynamic downcasting of
/// errors for use in highly generic contexts.
///
/// ## When to use this trait
///
/// In the vast majority of cases, a library-specific `enum` should be used
/// for cases where there can be many different types of errors. This has
/// the benefit of being very performant and benefiting from all sorts
/// of static checking at both the instantiation site and the handling
/// site of the error.
///
/// In other cases, being generic over `std::error::Error` may be correct
/// - usually for logging errors or in other places where an error is
/// used as *input*.
///
/// Now, a motivating example for this trait, which doesn't fall under
/// either of these cases:
///
/// Imagine we are creating a simple web middleware for verifying incoming
/// HTTP requests. It will take in many different user-defined `Verifier`s
/// and will call them one after the other, rejecting the request on any
/// error.
///
/// The first step would be to write a `Verifier` trait:
///
/// ```ignore
/// # struct Request;
/// pub trait Verifier {
///     /// Verify the request, yielding an error if the request is invalid.
///     fn verify(&Request) -> Result<(), ???>;
/// }
/// ```
///
/// A problem quickly arises - what type do we use for the `Err` case? We
/// cannot use a concrete type since each `Verifier` may wish to throw
/// any number of different errors, and we cannot use a generic since
/// the type is chosen by the implementor, not the caller, and it cannot
/// be a generic on the trait since we will want to store many `Verifier`s
/// together.
///
/// Enter: `Box<error::Error>`, a type which can be used to represent
/// any `std::error::Error` with the sufficient bounds, and can *also*
/// be handled later by downcasting it to the right error using either
/// `.downcast` or the `match_error!` macro. This type can be used to meet
/// the needs of consumers like `Verifier`, but should not be used in cases
/// where enums or generics are better suited.
pub trait Error: Show + Send + Typeable + StdError { }

impl<S: StdError + Show + Send + Typeable> Error for S { }

impl Error {
    /// Is this `Error` object of type `E`?
    pub fn is<E: Error>(&self) -> bool { self.get_type() == TypeId::of::<E>() }

    /// If this error is `E`, downcast this error to `E`, by reference.
    pub fn downcast<E: Error>(&self) -> Option<&E> {
        if self.is::<E>() {
            unsafe { Some(mem::transmute(
                mem::transmute::<&Error, raw::TraitObject>(self).data
            )) }
        } else {
            None
        }
    }
}

impl Show for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { (*self).fmt(f) }
}

impl<E: Error> FromError<E> for Box<Error> {
    fn from_error(e: E) -> Box<Error> { box e }
}

#[macro_export]
macro_rules! match_error {
    ($m:expr, $i1:pat: $t1:ty => $e1:expr) => {{
        let tmp = $m;
        match tmp.downcast::<$t1>() {
            Some($i1) => Some($e1),
            None => None,
        }
    }};

    ($m:expr, $i1:pat: $t1:ty => $e1:expr, $($i:pat: $t:ty => $e:expr),+) => {{
        let tmp = $m;
        match tmp.downcast::<$t1>() {
            Some($i1) => Some($e1),
            None => match_error!(tmp, $($i: $t => $e),*),
        }
    }};
}

#[cfg(test)]
mod test {
    use super::Error;
    use std::error::Error as StdError;

    #[deriving(Show, PartialEq)]
    pub struct ParseError {
        location: uint,
    }

    impl StdError for ParseError {
        fn description(&self) -> &str { "Parse Error" }
    }

    #[test] fn test_generic() {
        fn produce_parse_error() -> Box<Error> {
            box ParseError { location: 7u }
        }

        fn generic_handler(raw: Box<Error>) {
            (match_error! { raw,
                parse: ParseError => {
                    assert_eq!(*parse, ParseError { location: 7u })
                }
            }).unwrap()
        }

        generic_handler(produce_parse_error())
    }
}

