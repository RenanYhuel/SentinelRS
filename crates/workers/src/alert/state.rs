use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RuleState {
    Ok,
    Pending { since_ms: i64 },
    Firing { since_ms: i64 },
    Resolved { at_ms: i64 },
}

impl RuleState {
    pub fn transition(self, condition_met: bool, now_ms: i64, for_duration_ms: i64) -> Self {
        match (self, condition_met) {
            (Self::Ok, true) => {
                if for_duration_ms == 0 {
                    Self::Firing { since_ms: now_ms }
                } else {
                    Self::Pending { since_ms: now_ms }
                }
            }
            (Self::Ok, false) => Self::Ok,

            (Self::Pending { since_ms }, true) => {
                if now_ms - since_ms >= for_duration_ms {
                    Self::Firing { since_ms }
                } else {
                    Self::Pending { since_ms }
                }
            }
            (Self::Pending { .. }, false) => Self::Ok,

            (Self::Firing { since_ms }, true) => Self::Firing { since_ms },
            (Self::Firing { .. }, false) => Self::Resolved { at_ms: now_ms },

            (Self::Resolved { .. }, true) => {
                if for_duration_ms == 0 {
                    Self::Firing { since_ms: now_ms }
                } else {
                    Self::Pending { since_ms: now_ms }
                }
            }
            (Self::Resolved { .. }, false) => Self::Ok,
        }
    }

    pub fn is_firing(&self) -> bool {
        matches!(self, Self::Firing { .. })
    }

    pub fn just_resolved(&self) -> bool {
        matches!(self, Self::Resolved { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_to_firing_no_duration() {
        let s = RuleState::Ok.transition(true, 1000, 0);
        assert!(s.is_firing());
    }

    #[test]
    fn ok_to_pending_with_duration() {
        let s = RuleState::Ok.transition(true, 1000, 5000);
        assert!(matches!(s, RuleState::Pending { since_ms: 1000 }));
    }

    #[test]
    fn pending_to_firing_after_duration() {
        let s = RuleState::Pending { since_ms: 1000 }.transition(true, 6000, 5000);
        assert!(s.is_firing());
    }

    #[test]
    fn pending_resets_on_false() {
        let s = RuleState::Pending { since_ms: 1000 }.transition(false, 2000, 5000);
        assert!(matches!(s, RuleState::Ok));
    }

    #[test]
    fn firing_to_resolved() {
        let s = RuleState::Firing { since_ms: 1000 }.transition(false, 7000, 5000);
        assert!(s.just_resolved());
    }

    #[test]
    fn resolved_to_ok() {
        let s = RuleState::Resolved { at_ms: 7000 }.transition(false, 8000, 0);
        assert!(matches!(s, RuleState::Ok));
    }
}
