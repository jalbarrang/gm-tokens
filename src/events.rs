use crate::domain::{CareerPhase, GoddessColor, TrainingType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEvent {
    TrainingClick {
        turn: u32,
        training: TrainingType,
    },
    FragmentGained {
        turn: u32,
        color: GoddessColor,
        training: TrainingType,
        source: crate::domain::FragmentSource,
    },
    GmRaceCompleted {
        turn: u32,
        won: bool,
    },
    BurstActivated {
        turn: u32,
        color: GoddessColor,
    },
    PhaseStarted {
        turn: u32,
        phase: CareerPhase,
    },
}
