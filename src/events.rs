//! The event stream a host records; all state is rebuilt from it.

use crate::domain::{CareerPhase, GoddessColor, TrainingType};
use serde::{Deserialize, Serialize};

/// A single observed fact about the run. Recording events is the only way
/// state enters the crate, so a serialized event list replays identically in
/// any host.
///
/// A double fragment drop is two [`RunEvent::FragmentGained`] on the same
/// turn; a training that fails yields neither a click nor a fragment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEvent {
    /// A successful training click; worth 2 facility points in that training.
    TrainingClick {
        turn: u32,
        training: TrainingType,
    },
    /// One fragment entering the tree.
    FragmentGained {
        turn: u32,
        color: GoddessColor,
        training: TrainingType,
        source: crate::domain::FragmentSource,
    },
    /// A GM race result; worth 8 points to every facility on a win, 4 on a loss.
    GmRaceCompleted {
        turn: u32,
        won: bool,
    },
    /// A goddess burst was activated; consumes the completed tree and levels
    /// that goddess.
    BurstActivated {
        turn: u32,
        color: GoddessColor,
    },
    /// The run entered a new career phase.
    PhaseStarted {
        turn: u32,
        phase: CareerPhase,
    },
}
