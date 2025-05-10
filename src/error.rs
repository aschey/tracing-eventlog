use thiserror::Error;
use widestring::error::ContainsNul;

pub type Result<T> = core::result::Result<T, EventLogError>;

#[derive(Error, Debug)]
pub enum EventLogError {
    #[error("Invalid string: {0}")]
    StrConvertError(#[from] ContainsNul<u16>),
    #[cfg(windows)]
    #[error("Error invoking windows API: {0}")]
    WindowsError(#[from] windows::core::Error),
    #[cfg(not(windows))]
    #[error("")]
    WindowsError(String),
    #[error("OS error occured during Windows API call: {0}")]
    SystemError(#[from] std::io::Error),
}

#[cfg(windows)]
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("OS error occured during Windows API call: {0}")]
    SystemError(#[from] std::io::Error),
    #[error("Unable to locate current exe path")]
    InvalidExePath,
    #[error("Permission denied: {0}")]
    PermissionDenied(::windows::core::Error),
    #[error("Error settings registry key: {0}")]
    KeyError(::windows::core::Error),
    #[error("Error setting registry value: {0}")]
    ValueError(::windows::core::Error),
    #[error("Invalid string: {0}")]
    StrConvertError(#[from] utfx::NulError<u16>),
}

#[cfg(not(windows))]
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("")]
    SystemError(String),
    #[error("")]
    InvalidExePath(String),
    #[error("")]
    PermissionDenied(String),
    #[error("")]
    KeyError(String),
    #[error("")]
    ValueError(String),
    #[error("")]
    StrConvertError(String),
}
