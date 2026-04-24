//! Door Lock Controller
//!
//! Implements core control logic for door locks, including command processing, hardware control and event management

use embassy_time::{Duration, Timer};
use esp_idf_svc::hal::gpio::{AnyOutputPin, Gpio5, Gpio6, Gpio7, Input, Output, PinDriver};

pub struct LockController {
    /// The lock output pin
    lock: PinDriver<'static, AnyOutputPin, Output>,

    /// State
    is_locked: bool,
}

impl LockController {
    pub fn new(lock: PinDriver<'static, AnyOutputPin, Output>) -> Self {
        Self { lock }
    }
}
