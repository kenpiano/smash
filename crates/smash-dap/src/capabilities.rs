//! DAP capabilities tracking.

use crate::protocol::Capabilities;

/// Resolved capabilities of the debug adapter, stored as plain booleans.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DapCapabilities {
    /// Whether the adapter supports `configurationDone`.
    pub supports_configuration_done_request: bool,
    /// Whether the adapter supports conditional breakpoints.
    pub supports_conditional_breakpoints: bool,
    /// Whether the adapter supports hit-count breakpoints.
    pub supports_hit_conditional_breakpoints: bool,
    /// Whether the adapter supports `evaluate` for hovers.
    pub supports_evaluate_for_hovers: bool,
    /// Whether the adapter supports stepping backwards.
    pub supports_step_back: bool,
    /// Whether the adapter supports setting variable values.
    pub supports_set_variable: bool,
    /// Whether the adapter supports the `terminate` request.
    pub supports_terminate_request: bool,
}

impl DapCapabilities {
    /// Build [`DapCapabilities`] from the protocol-level [`Capabilities`]
    /// returned by the adapter in the `initialize` response.
    pub fn from_initialize_response(caps: &Capabilities) -> Self {
        Self {
            supports_configuration_done_request: caps
                .supports_configuration_done_request
                .unwrap_or(false),
            supports_conditional_breakpoints: caps
                .supports_conditional_breakpoints
                .unwrap_or(false),
            supports_hit_conditional_breakpoints: caps
                .supports_hit_conditional_breakpoints
                .unwrap_or(false),
            supports_evaluate_for_hovers: caps.supports_evaluate_for_hovers.unwrap_or(false),
            supports_step_back: caps.supports_step_back.unwrap_or(false),
            supports_set_variable: caps.supports_set_variable.unwrap_or(false),
            supports_terminate_request: caps.supports_terminate_request.unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Capabilities;

    #[test]
    fn capabilities_from_full_response() {
        let caps = Capabilities {
            supports_configuration_done_request: Some(true),
            supports_conditional_breakpoints: Some(true),
            supports_hit_conditional_breakpoints: Some(false),
            supports_evaluate_for_hovers: Some(true),
            supports_step_back: Some(false),
            supports_set_variable: Some(true),
            supports_terminate_request: Some(true),
        };
        let resolved = DapCapabilities::from_initialize_response(&caps);
        assert!(resolved.supports_configuration_done_request);
        assert!(resolved.supports_conditional_breakpoints);
        assert!(!resolved.supports_hit_conditional_breakpoints);
        assert!(resolved.supports_evaluate_for_hovers);
        assert!(!resolved.supports_step_back);
        assert!(resolved.supports_set_variable);
        assert!(resolved.supports_terminate_request);
    }

    #[test]
    fn capabilities_from_empty_response() {
        let caps = Capabilities::default();
        let resolved = DapCapabilities::from_initialize_response(&caps);
        assert!(!resolved.supports_configuration_done_request);
        assert!(!resolved.supports_conditional_breakpoints);
        assert!(!resolved.supports_hit_conditional_breakpoints);
        assert!(!resolved.supports_evaluate_for_hovers);
        assert!(!resolved.supports_step_back);
        assert!(!resolved.supports_set_variable);
        assert!(!resolved.supports_terminate_request);
    }

    #[test]
    fn capabilities_partial_response() {
        let caps = Capabilities {
            supports_configuration_done_request: Some(true),
            supports_conditional_breakpoints: None,
            supports_hit_conditional_breakpoints: None,
            supports_evaluate_for_hovers: None,
            supports_step_back: None,
            supports_set_variable: None,
            supports_terminate_request: Some(true),
        };
        let resolved = DapCapabilities::from_initialize_response(&caps);
        assert!(resolved.supports_configuration_done_request);
        assert!(!resolved.supports_conditional_breakpoints);
        assert!(resolved.supports_terminate_request);
    }

    #[test]
    fn capabilities_default_is_all_false() {
        let d = DapCapabilities::default();
        assert!(!d.supports_configuration_done_request);
        assert!(!d.supports_conditional_breakpoints);
        assert!(!d.supports_hit_conditional_breakpoints);
        assert!(!d.supports_evaluate_for_hovers);
        assert!(!d.supports_step_back);
        assert!(!d.supports_set_variable);
        assert!(!d.supports_terminate_request);
    }

    #[test]
    fn capabilities_clone_and_eq() {
        let caps = DapCapabilities {
            supports_configuration_done_request: true,
            supports_conditional_breakpoints: false,
            ..Default::default()
        };
        let cloned = caps.clone();
        assert_eq!(caps, cloned);
    }
}
