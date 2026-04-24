mod controller;
mod state;

pub use controller::LockController;

/// Door lock control interface
pub trait LockControl {
    /// Unlock the door lock
    async fn unlock(&mut self, source: UnlockSource) -> Result<(), LockError>;

    /// Lock the door lock
    async fn lock(&mut self) -> Result<(), LockError>;

    /// Get current status
    fn get_status(&self) -> LockStatus;

    /// Set working mode
    async fn set_mode(&mut self, mode: LockMode) -> Result<(), LockError>;
}

/// Unlock source
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnlockSource {
    /// NFC card ID
    Nfc(u32),
    /// APP user ID
    App(u32),
    /// Bluetooth device ID
    Bluetooth(u32),
    /// Remote command ID
    Remote(u32),
    /// Emergency unlock
    Emergency,
    /// Manual unlock
    Manual,
}
