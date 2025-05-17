//! Helpers/utilities in form of macros.

/// Check if the `Option` has a value, and if not, return an [`OsError::UnexpectedNull`](crate::OsError::UnexpectedNull).
#[macro_export]
macro_rules! null_check {
    ($e: expr) => {
        $e.ok_or($crate::OsError::UnexpectedNull)?
    };
}

/// Remap a `Result<T, EspError>` into a varint of [`OsError`](crate::OsError), while
/// also keeping the [`EspError`](esp_idf_svc::sys::EspError) information.
#[macro_export]
macro_rules! re_esp {
    ($e: expr, $n: ident) => {
        $e.map_err(|e| $crate::OsError::$n(e))
    };
}
