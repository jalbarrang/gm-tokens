use crate::domain::{CareerPhase, GoddessColor, TrainingType};
use serde::{Deserialize, Serialize};

/// One recorded fact about the run; the event list is the crate's only input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEvent {
    /// A successful training click: 2 facility points in that training.
    TrainingClick {
        turn: u32,
        training: TrainingType,
    },
    /// One fragment gained; a double drop is two of these on one turn.
    FragmentGained {
        turn: u32,
        color: GoddessColor,
        training: TrainingType,
        source: crate::domain::FragmentSource,
    },
    /// GM race result: 8 points to every facility on a win, 4 on a loss.
    GmRaceCompleted {
        turn: u32,
        won: bool,
    },
    /// A goddess burst: consumes the completed tree, levels that goddess.
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
