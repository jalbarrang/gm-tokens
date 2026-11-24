use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoddessColor {
    Red,
    Blue,
    Yellow,
}

impl GoddessColor {
    pub const COUNT: usize = 3;

    pub fn index(self) -> usize {
        match self {
            GoddessColor::Red => 0,
            GoddessColor::Blue => 1,
            GoddessColor::Yellow => 2,
        }
    }

    pub const ALL: [GoddessColor; GoddessColor::COUNT] =
        [GoddessColor::Red, GoddessColor::Blue, GoddessColor::Yellow];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingType {
    Speed,
    Stamina,
    Power,
    Guts,
    Wit,
    SkillPoints,
}

impl TrainingType {
    pub const COUNT: usize = 6;

    pub fn index(self) -> usize {
        match self {
            TrainingType::Speed => 0,
            TrainingType::Stamina => 1,
            TrainingType::Power => 2,
            TrainingType::Guts => 3,
            TrainingType::Wit => 4,
            TrainingType::SkillPoints => 5,
        }
    }

    pub const ALL: [TrainingType; TrainingType::COUNT] = [
        TrainingType::Speed,
        TrainingType::Stamina,
        TrainingType::Power,
        TrainingType::Guts,
        TrainingType::Wit,
        TrainingType::SkillPoints,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentSource {
    Training,
    TargetRace,
    NormalRace,
    Rest,
    Date,
    Ancestor,
    GoddessEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CareerPhase {
    Junior,
    Classic,
    Senior,
    UraFinale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FragmentToken {
    pub color: GoddessColor,
    pub training: TrainingType,
    pub source: FragmentSource,
    pub turn: u32,
}
