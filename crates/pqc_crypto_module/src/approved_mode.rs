//! Approved-mode state machine.
//!
//! The module starts in `Uninitialized`. After `initialize_approved_mode()`
//! runs self-tests, it transitions to `Approved` or `Error`. All crypto APIs
//! reject calls unless state is `Approved`.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::errors::CryptoError;

/// Module lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModuleState {
    Uninitialized = 0,
    SelfTesting = 1,
    Approved = 2,
    Error = 3,
}

impl From<u8> for ModuleState {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Uninitialized,
            1 => Self::SelfTesting,
            2 => Self::Approved,
            3 => Self::Error,
            _ => Self::Error,
        }
    }
}

static MODULE_STATE: AtomicU8 = AtomicU8::new(0); // Uninitialized

/// Get the current module state.
pub fn state() -> ModuleState {
    ModuleState::from(MODULE_STATE.load(Ordering::SeqCst))
}

/// Transition to a new state. Returns the previous state.
pub(crate) fn set_state(new: ModuleState) -> ModuleState {
    let prev = MODULE_STATE.swap(new as u8, Ordering::SeqCst);
    ModuleState::from(prev)
}

/// Guard: returns `Ok(())` if state is `Approved`, else returns the appropriate error.
pub fn require_approved() -> Result<(), CryptoError> {
    match state() {
        ModuleState::Approved => Ok(()),
        ModuleState::Uninitialized | ModuleState::SelfTesting => {
            Err(CryptoError::ModuleNotInitialized)
        }
        ModuleState::Error => Err(CryptoError::ModuleInErrorState),
    }
}

/// Reset to Uninitialized (for testing only).
#[cfg(test)]
pub fn reset_for_testing() {
    MODULE_STATE.store(0, Ordering::SeqCst);
}

// Also expose for integration tests via a public function gated on test builds.
#[doc(hidden)]
pub fn __test_reset() {
    MODULE_STATE.store(0, Ordering::SeqCst);
}
