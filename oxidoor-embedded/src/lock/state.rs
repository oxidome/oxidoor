//! Door lock state management
//!
//! Implements the door lock state machine, manages state transitions and state persistence

use serde::{Deserialize, Serialize};
use embassy_time::{Duration, Instant};
use heapless::Vec;
use common::error::LockError;

/// Door lock state
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LockState {
    /// Locked state
    Locked,
    /// Unlocked state
    Unlocked,
    /// Unlocking (transition state)
    Unlocking,
    /// Locking (transition state)
    Locking,
    /// Fault state
    Fault(FaultReason),
    /// Maintenance mode
    Maintenance,
    /// Emergency unlock
    Emergency,
    /// Tamper alert
    TamperAlert,
}

/// Fault reason
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FaultReason {
    /// Motor fault
    MotorFault,
    /// Sensor fault
    SensorFault,
    /// Communication fault
    CommunicationFault,
    /// Power fault
    PowerFault,
    /// Mechanical fault
    MechanicalFault,
    /// Unknown fault
    Unknown,
}

/// Door lock working mode
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LockMode {
    /// Normal mode
    Normal,
    /// Always open mode (office hours)
    AlwaysOpen,
    /// Always closed mode (security mode)
    AlwaysClosed,
    /// One-time mode (auto lock after opening)
    OneTime,
    /// Scheduled mode (work by schedule)
    Scheduled,
    /// Double authentication mode
    DoubleAuth,
}

/// Complete door lock status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockStatus {
    /// Current state
    pub state: LockState,
    /// Working mode
    pub mode: LockMode,
    /// Last state change time
    pub last_change: Instant,
    /// Last unlock source
    pub last_unlock_source: Option<super::UnlockSource>,
    /// Daily unlock count
    pub daily_unlock_count: u32,
    /// Consecutive failed attempts
    pub failed_attempts: u8,
    /// Battery level (percentage)
    pub battery_level: Option<u8>,
    /// Physical door status
    pub door_status: DoorStatus,
}

/// Physical door status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DoorStatus {
    /// Door closed
    Closed,
    /// Door open
    Open,
    /// Door ajar
    Ajar,
    /// Unknown status
    Unknown,
}

/// State transition rule
pub struct StateTransition {
    /// From state
    pub from: LockState,
    /// To state
    pub to: LockState,
    /// Transition condition
    pub condition: TransitionCondition,
}

/// State transition condition
#[derive(Debug, Clone)]
pub enum TransitionCondition {
    /// Authentication success
    AuthSuccess,
    /// Timeout
    Timeout,
    /// Manual trigger
    Manual,
    /// Auto trigger
    Auto,
    /// Emergency
    Emergency,
    /// Fault recovery
    FaultRecovery,
}

/// State machine manager
pub struct StateMachine {
    current_state: LockState,
    mode: LockMode,
    transitions: Vec<StateTransition, 32>,
    history: Vec<StateChange, 100>,
    state_timeout: Option<StateTimeout>,
}

/// State change record
#[derive(Debug, Clone)]
struct StateChange {
    from: LockState,
    to: LockState,
    timestamp: Instant,
    reason: TransitionCondition,
}

/// State timeout configuration
#[derive(Debug, Clone)]
struct StateTimeout {
    state: LockState,
    duration: Duration,
    target_state: LockState,
}

impl StateMachine {
    /// Create new state machine
    pub fn new() -> Self {
        let mut sm = Self {
            current_state: LockState::Locked,
            mode: LockMode::Normal,
            transitions: Vec::new(),
            history: Vec::new(),
            state_timeout: None,
        };
        
        // Initialize state transition rules
        sm.init_transitions();
        sm
    }
    
    /// Initialize state transition rules
    fn init_transitions(&mut self) {
        // Define allowed state transitions
        let transitions = [
            // Normal unlock flow
            StateTransition {
                from: LockState::Locked,
                to: LockState::Unlocking,
                condition: TransitionCondition::AuthSuccess,
            },
            StateTransition {
                from: LockState::Unlocking,
                to: LockState::Unlocked,
                condition: TransitionCondition::Auto,
            },
            // Normal lock flow
            StateTransition {
                from: LockState::Unlocked,
                to: LockState::Locking,
                condition: TransitionCondition::Auto,
            },
            StateTransition {
                from: LockState::Locking,
                to: LockState::Locked,
                condition: TransitionCondition::Auto,
            },
            // Emergency unlock
            StateTransition {
                from: LockState::Locked,
                to: LockState::Emergency,
                condition: TransitionCondition::Emergency,
            },
            // Fault handling
            StateTransition {
                from: LockState::Unlocking,
                to: LockState::Fault(FaultReason::Unknown),
                condition: TransitionCondition::Timeout,
            },
            StateTransition {
                from: LockState::Locking,
                to: LockState::Fault(FaultReason::Unknown),
                condition: TransitionCondition::Timeout,
            },
        ];
        
        for transition in transitions {
            let _ = self.transitions.push(transition);
        }
    }
    
    /// Try state transition
    pub fn try_transition(
        &mut self,
        target: LockState,
        condition: TransitionCondition,
    ) -> Result<(), LockError> {
        // Check if transition is allowed
        let valid = self.transitions.iter().any(|t| {
            match (t.from, self.current_state) {
                (LockState::Fault(_), LockState::Fault(_)) => true,
                (from, current) if from == current => {
                    match (t.to, target) {
                        (LockState::Fault(_), LockState::Fault(_)) => true,
                        (to, tgt) => to == tgt,
                    }
                }
                _ => false,
            }
        });
        
        if !valid {
            return Err(LockError::InvalidStateTransition);
        }
        
        // Record state change
        let change = StateChange {
            from: self.current_state,
            to: target,
            timestamp: Instant::now(),
            reason: condition,
        };
        
        // Save history (circular buffer)
        if self.history.is_full() {
            self.history.remove(0);
        }
        let _ = self.history.push(change);
        
        // Update state
        self.current_state = target;
        
        // Set state timeout
        self.setup_timeout(target);
        
        Ok(())
    }
    
    /// Set state timeout
    fn setup_timeout(&mut self, state: LockState) {
        self.state_timeout = match state {
            LockState::Unlocked => Some(StateTimeout {
                state,
                duration: Duration::from_millis(5000),
                target_state: LockState::Locking,
            }),
            LockState::Unlocking | LockState::Locking => Some(StateTimeout {
                state,
                duration: Duration::from_millis(3000),
                target_state: LockState::Fault(FaultReason::Unknown),
            }),
            _ => None,
        };
    }
    
    /// Check and handle timeout
    pub async fn check_timeout(&mut self) -> Option<LockState> {
        if let Some(ref timeout) = self.state_timeout {
            if self.current_state == timeout.state {
                // Should check actual timeout here
                // Simplified example, should compare with timeout.duration
                return Some(timeout.target_state);
            }
        }
        None
    }
    
    /// Get current state
    pub fn current(&self) -> LockState {
        self.current_state
    }
    
    /// Get working mode
    pub fn mode(&self) -> LockMode {
        self.mode
    }
    
    /// Set working mode
    pub fn set_mode(&mut self, mode: LockMode) -> Result<(), LockError> {
        // Mode switching not allowed in certain states
        match self.current_state {
            LockState::Fault(_) | LockState::Emergency => {
                return Err(LockError::OperationNotAllowed);
            }
            _ => {}
        }
        
        self.mode = mode;
        Ok(())
    }
    
    /// Is in secure state (locked)
    pub fn is_secure(&self) -> bool {
        matches!(
            self.current_state,
            LockState::Locked | LockState::Locking | LockState::Maintenance
        )
    }
    
    /// Get state history
    pub fn get_history(&self) -> &[StateChange] {
        &self.history
    }
    
    /// Clear fault state
    pub fn clear_fault(&mut self) -> Result<(), LockError> {
        if matches!(self.current_state, LockState::Fault(_)) {
            self.current_state = LockState::Locked;
            Ok(())
        } else {
            Err(LockError::InvalidStateTransition)
        }
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}
