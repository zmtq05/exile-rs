use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

pub mod version;

pub mod error;
pub mod google_drive;
pub mod manager;
pub mod progress;

/// Holds the active installation's cancellation token (if any).
/// Used to safely cancel ongoing install operations.
#[derive(Debug, Default)]
pub struct InstallCancelToken(Mutex<Option<CancellationToken>>);

impl InstallCancelToken {
    /// Store a new cancellation token for the current install.
    pub fn set(&self, token: CancellationToken) {
        *self.0.lock().unwrap() = Some(token);
    }

    /// Clear the stored token (call on install completion).
    pub fn take(&self) -> Option<CancellationToken> {
        self.0.lock().unwrap().take()
    }

    /// Cancel the current install if one is in progress.
    pub fn cancel(&self) {
        if let Some(token) = self.0.lock().unwrap().as_ref() {
            token.cancel();
        }
    }
}
