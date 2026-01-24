use std::{ops::Deref, sync::atomic::AtomicBool};

pub mod version;

pub mod error;
pub mod google_drive;
pub mod manager;
pub mod progress;

#[derive(Debug, Default)]
pub struct Installing(AtomicBool);

impl Deref for Installing {
    type Target = AtomicBool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
