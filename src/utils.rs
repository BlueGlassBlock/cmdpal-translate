use windows::core::Error as WinError;
use windows::Win32::Foundation::{E_FAIL, ERROR_LOCK_VIOLATION};

pub fn map_lock_err<E: ToString>(e: E) -> WinError {
    WinError::new(ERROR_LOCK_VIOLATION.into(), e.to_string())
}

pub fn map_fail_err<E: ToString>(e: E) -> WinError {
    WinError::new(E_FAIL.into(), e.to_string())
}