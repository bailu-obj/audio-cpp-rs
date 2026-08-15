//! Error types for the safe `audio.cpp` wrapper.

use std::ffi::NulError;
use std::path::PathBuf;
use std::str::Utf8Error;

use audio_cpp_sys::audio_cpp_status;

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Classification of a native `audio.cpp` failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// A required argument was missing or malformed.
    InvalidArgument,
    /// A model, family, or asset could not be found.
    NotFound,
    /// The requested task, backend, or mode is unsupported.
    Unsupported,
    /// A native runtime exception or internal failure.
    Runtime,
    /// Native allocation failed.
    OutOfMemory,
}

impl ErrorKind {
    pub(crate) fn from_status(status: audio_cpp_status) -> Option<Self> {
        match status {
            audio_cpp_status::AUDIO_CPP_OK => None,
            audio_cpp_status::AUDIO_CPP_ERR_INVALID_ARG => Some(Self::InvalidArgument),
            audio_cpp_status::AUDIO_CPP_ERR_NOT_FOUND => Some(Self::NotFound),
            audio_cpp_status::AUDIO_CPP_ERR_UNSUPPORTED => Some(Self::Unsupported),
            audio_cpp_status::AUDIO_CPP_ERR_OOM => Some(Self::OutOfMemory),
            audio_cpp_status::AUDIO_CPP_ERR_RUNTIME => Some(Self::Runtime),
        }
    }
}

/// Failures from the safe `audio.cpp` bindings.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Native library error with a copied message.
    #[error("{kind:?}: {message}")]
    Native {
        /// Native error class.
        kind: ErrorKind,
        /// Message from `audio_cpp_last_error`.
        message: String,
    },
    /// A Rust string or path contained an interior NUL.
    #[error("value contains an interior NUL byte")]
    Nul(#[from] NulError),
    /// Native text was not valid UTF-8.
    #[error("native text was not valid UTF-8")]
    Utf8(#[from] Utf8Error),
    /// A filesystem path was not valid Unicode.
    #[error("path is not valid Unicode: {}", .0.display())]
    InvalidPath(PathBuf),
    /// The loaded session does not implement the requested run mode.
    #[error("{0}")]
    Unsupported(String),
}

impl Error {
    pub(crate) fn native(status: audio_cpp_status, message: impl Into<String>) -> Self {
        Self::Native {
            kind: ErrorKind::from_status(status).unwrap_or(ErrorKind::Runtime),
            message: message.into(),
        }
    }

    pub(crate) fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported(message.into())
    }
}
