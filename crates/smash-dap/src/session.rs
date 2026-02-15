//! DAP session state machine.

use crate::capabilities::DapCapabilities;
use crate::error::DapError;
use crate::protocol::Capabilities;

/// The current state of a debug session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session has been created but not initialized.
    Uninitialized,
    /// The `initialize` handshake has completed.
    Initialized,
    /// A launch/attach has been performed; the debuggee is running.
    Running,
    /// The debuggee is stopped (e.g. at a breakpoint).
    Stopped,
    /// The session has been terminated/disconnected.
    Terminated,
}

/// Manages the lifecycle state of a single debug session.
#[derive(Debug)]
pub struct DapSession {
    state: SessionState,
    capabilities: DapCapabilities,
    next_seq: i64,
}

impl DapSession {
    /// Create a new session in the [`Uninitialized`](SessionState::Uninitialized) state.
    pub fn new() -> Self {
        Self {
            state: SessionState::Uninitialized,
            capabilities: DapCapabilities::default(),
            next_seq: 1,
        }
    }

    /// Return the current session state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Return the resolved adapter capabilities.
    pub fn capabilities(&self) -> &DapCapabilities {
        &self.capabilities
    }

    /// Allocate the next sequence number for an outgoing request.
    pub fn next_seq(&mut self) -> i64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }

    /// Transition: Uninitialized → Initialized.
    ///
    /// Call after receiving a successful `initialize` response.
    pub fn initialize(&mut self, caps: &Capabilities) -> Result<(), DapError> {
        self.require_not_terminated()?;
        if self.state != SessionState::Uninitialized {
            return Err(DapError::Rejected {
                message: format!("cannot initialize: session is in {:?} state", self.state),
            });
        }
        self.capabilities = DapCapabilities::from_initialize_response(caps);
        self.state = SessionState::Initialized;
        Ok(())
    }

    /// Transition: Initialized → Running (via launch).
    pub fn launch(&mut self) -> Result<(), DapError> {
        self.require_initialized_or_stopped("launch")?;
        if self.state != SessionState::Initialized {
            return Err(DapError::Rejected {
                message: format!("cannot launch: session is in {:?} state", self.state),
            });
        }
        self.state = SessionState::Running;
        Ok(())
    }

    /// Transition: Initialized → Running (via attach).
    pub fn attach(&mut self) -> Result<(), DapError> {
        self.require_initialized_or_stopped("attach")?;
        if self.state != SessionState::Initialized {
            return Err(DapError::Rejected {
                message: format!("cannot attach: session is in {:?} state", self.state),
            });
        }
        self.state = SessionState::Running;
        Ok(())
    }

    /// Transition: Running → Stopped (when a stopped event is received).
    pub fn handle_stopped(&mut self) -> Result<(), DapError> {
        self.require_not_terminated()?;
        if self.state != SessionState::Running {
            return Err(DapError::Rejected {
                message: format!("cannot stop: session is in {:?} state", self.state),
            });
        }
        self.state = SessionState::Stopped;
        Ok(())
    }

    /// Transition: Stopped → Running (via continue).
    pub fn continue_execution(&mut self) -> Result<(), DapError> {
        self.require_not_terminated()?;
        if self.state != SessionState::Stopped {
            return Err(DapError::Rejected {
                message: format!("cannot continue: session is in {:?} state", self.state),
            });
        }
        self.state = SessionState::Running;
        Ok(())
    }

    /// Transition: any → Terminated (via disconnect).
    pub fn disconnect(&mut self) -> Result<(), DapError> {
        self.require_not_terminated()?;
        self.state = SessionState::Terminated;
        Ok(())
    }

    /// Check that the session is not terminated.
    fn require_not_terminated(&self) -> Result<(), DapError> {
        if self.state == SessionState::Terminated {
            return Err(DapError::Terminated);
        }
        Ok(())
    }

    /// Check that the session is at least initialized (not uninitialized or terminated).
    fn require_initialized_or_stopped(&self, op: &str) -> Result<(), DapError> {
        match self.state {
            SessionState::Uninitialized => Err(DapError::NotInitialized),
            SessionState::Terminated => Err(DapError::Terminated),
            _ => {
                // Caller does further state checks.
                let _ = op;
                Ok(())
            }
        }
    }
}

impl Default for DapSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Capabilities;

    fn sample_caps() -> Capabilities {
        Capabilities {
            supports_configuration_done_request: Some(true),
            supports_conditional_breakpoints: Some(true),
            ..Default::default()
        }
    }

    #[test]
    fn session_lifecycle_happy_path() {
        let mut session = DapSession::new();
        assert_eq!(session.state(), SessionState::Uninitialized);

        // Initialize.
        session.initialize(&sample_caps()).unwrap();
        assert_eq!(session.state(), SessionState::Initialized);
        assert!(session.capabilities().supports_configuration_done_request);

        // Launch.
        session.launch().unwrap();
        assert_eq!(session.state(), SessionState::Running);

        // Stopped event.
        session.handle_stopped().unwrap();
        assert_eq!(session.state(), SessionState::Stopped);

        // Continue.
        session.continue_execution().unwrap();
        assert_eq!(session.state(), SessionState::Running);

        // Another stop + disconnect.
        session.handle_stopped().unwrap();
        session.disconnect().unwrap();
        assert_eq!(session.state(), SessionState::Terminated);
    }

    #[test]
    fn session_operations_before_init() {
        let mut session = DapSession::new();

        // Cannot launch before init.
        let err = session.launch().unwrap_err();
        assert!(matches!(err, DapError::NotInitialized));

        // Cannot attach before init.
        let err = session.attach().unwrap_err();
        assert!(matches!(err, DapError::NotInitialized));
    }

    #[test]
    fn session_operations_after_disconnect() {
        let mut session = DapSession::new();
        session.initialize(&sample_caps()).unwrap();
        session.disconnect().unwrap();

        // All operations should fail with Terminated.
        assert!(matches!(
            session.initialize(&sample_caps()),
            Err(DapError::Terminated)
        ));
        assert!(matches!(session.launch(), Err(DapError::Terminated)));
        assert!(matches!(session.attach(), Err(DapError::Terminated)));
        assert!(matches!(
            session.continue_execution(),
            Err(DapError::Terminated)
        ));
        assert!(matches!(
            session.handle_stopped(),
            Err(DapError::Terminated)
        ));
        assert!(matches!(session.disconnect(), Err(DapError::Terminated)));
    }

    #[test]
    fn session_sequence_tracking() {
        let mut session = DapSession::new();
        assert_eq!(session.next_seq(), 1);
        assert_eq!(session.next_seq(), 2);
        assert_eq!(session.next_seq(), 3);
    }

    #[test]
    fn session_double_initialize_rejected() {
        let mut session = DapSession::new();
        session.initialize(&sample_caps()).unwrap();
        let err = session.initialize(&sample_caps()).unwrap_err();
        assert!(matches!(err, DapError::Rejected { .. }));
    }

    #[test]
    fn session_cannot_continue_when_running() {
        let mut session = DapSession::new();
        session.initialize(&sample_caps()).unwrap();
        session.launch().unwrap();
        let err = session.continue_execution().unwrap_err();
        assert!(matches!(err, DapError::Rejected { .. }));
    }

    #[test]
    fn session_attach_happy_path() {
        let mut session = DapSession::new();
        session.initialize(&sample_caps()).unwrap();
        session.attach().unwrap();
        assert_eq!(session.state(), SessionState::Running);
    }

    #[test]
    fn session_default_trait() {
        let session = DapSession::default();
        assert_eq!(session.state(), SessionState::Uninitialized);
    }
}
