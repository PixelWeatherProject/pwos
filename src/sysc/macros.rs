//! Helpers/utilities in form of macros.

/// Check if the `Option` has a value, and if not, return an [`OsError::UnexpectedNull`](crate::OsError::UnexpectedNull).
#[macro_export]
macro_rules! null_check {
    ($e: expr) => {
        $e.ok_or($crate::OsError::UnexpectedNull)?
    };
}
