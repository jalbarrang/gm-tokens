//! Guide numbers (Lehan's GM guide, Taiwanese server); balance patches are edited here only.

use crate::domain::{CareerPhase, FragmentSource, GoddessColor, TrainingType};

/// Chance a source drops a second fragment; every rate here is from the
/// guide except `GoddessEvent`, an assumed placeholder the guide does not
/// state.
pub fn double_drop_chance(source: FragmentSource) -> f32 {
    match source {
        FragmentSource::TargetRace => 1.00,
        FragmentSource::Ancestor => 0.40,
        FragmentSource::Training
        | FragmentSource::NormalRace
        | FragmentSource::Rest
        | FragmentSource::Date => 0.20,
        FragmentSource::GoddessEvent => 0.20,
    }
}

pub const DOUBLE_DROP_RAINBOW_WIT: f32 = 0.40;
pub const DOUBLE_DROP_RAINBOW_NON_WIT: f32 = 1.00;

pub const POINTS_PER_TRAINING_CLICK: u32 = 2;
pub const POINTS_PER_FRAGMENT: u32 = 1;
pub const FRAGMENTS_PER_EXTRA_POINT: u32 = 5;
pub const RACE_WIN_POINTS: u32 = 8;
pub const RACE_LOSS_POINTS: u32 = 4;
pub const POINTS_PER_FACILITY_LEVEL: u32 = 12;

/// Per-training fragment drop rates, indexed by [`TrainingType::index`].
pub const ATTRIBUTE_DROP_RATES: [f32; TrainingType::COUNT] =
    [0.216, 0.106, 0.178, 0.128, 0.131, 0.241];

/// Rough energy-point value of a burst, as (min, max).
pub fn burst_value_estimate(color: GoddessColor) -> (u32, u32) {
    match color {
        GoddessColor::Red => (90, 100),
        GoddessColor::Blue => (70, 80),
        GoddessColor::Yellow => (50, 60),
    }
}

/// Aim for at most this many Yellow bursts per run; 4 is tolerated.
pub const YELLOW_BURST_CAP: u8 = 3;

/// The 3-4-4-1 burst plan.
pub fn expected_bursts(phase: CareerPhase) -> u8 {
    match phase {
        CareerPhase::Junior => 3,
        CareerPhase::Classic | CareerPhase::Senior => 4,
        CareerPhase::UraFinale => 1,
    }
}

/// Darley Arabian (Red): (energy discount %, training bonus %) per level.
pub const RED_PASSIVES: [(u8, u8); 5] = [(10, 5), (15, 8), (18, 11), (20, 13), (23, 15)];

/// Godolphin Barb (Blue): (hint rate up %, event chance up) per level.
pub const BLUE_PASSIVES: [(u8, u8); 5] =
    [(20, 20), (25, 25), (30, 30), (33, 33), (35, 35)];

/// Byerley Turk (Yellow): (event effect up %, chain chance up) per level.
pub const YELLOW_PASSIVES: [(u8, u8); 5] =
    [(10, 20), (15, 40), (20, 60), (23, 80), (25, 90)];

/// A burst's stat payout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatGain {
    pub speed: u16,
    pub stamina: u16,
    pub power: u16,
    pub guts: u16,
    pub wit: u16,
    pub skill_points: u16,
}

const fn gain(
    speed: u16,
    stamina: u16,
    power: u16,
    guts: u16,
    wit: u16,
    skill_points: u16,
) -> StatGain {
    StatGain { speed, stamina, power, guts, wit, skill_points }
}

/// Per-card payout of a Blue Burst on that training.
pub fn blue_burst_gains(training: TrainingType) -> StatGain {
    match training {
        TrainingType::Speed => gain(6, 0, 2, 0, 0, 20),
        TrainingType::Stamina => gain(0, 6, 0, 2, 0, 20),
        TrainingType::Power => gain(0, 2, 6, 0, 0, 20),
        TrainingType::Guts => gain(1, 0, 1, 6, 0, 20),
        TrainingType::Wit => gain(0, 0, 0, 0, 6, 25),
        TrainingType::SkillPoints => gain(0, 0, 0, 0, 0, 0),
    }
}

/// Red Burst training click payout, excluding the energy/mood/race bonus.
pub fn red_burst_gains(training: TrainingType) -> StatGain {
    match training {
        TrainingType::Speed => gain(15, 0, 6, 0, 0, 5),
        TrainingType::Stamina => gain(0, 13, 0, 9, 0, 5),
        TrainingType::Power => gain(0, 7, 14, 0, 0, 5),
        TrainingType::Guts => gain(4, 0, 5, 14, 0, 5),
        TrainingType::Wit => gain(5, 0, 0, 0, 13, 5),
        TrainingType::SkillPoints => gain(0, 0, 0, 0, 0, 5),
    }
}

/// A mid-run stat target; targets is Spd/Sta/Pow/Guts/Wit, None = deck-dependent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatCheckpoint {
    pub phase: CareerPhase,
    pub month: u8,
    pub targets: [Option<u16>; 5],
}

const fn checkpoint(phase: CareerPhase, month: u8, targets: [Option<u16>; 5]) -> StatCheckpoint {
    StatCheckpoint { phase, month, targets }
}

/// Stat checkpoints from the guide.
pub const STAT_CHECKPOINTS: &[StatCheckpoint] = &[
    checkpoint(CareerPhase::Classic, 6, [Some(600), Some(450), Some(450), None, Some(450)]),
    checkpoint(CareerPhase::Classic, 12, [Some(900), Some(600), Some(600), None, Some(600)]),
    checkpoint(CareerPhase::Senior, 6, [Some(1200), Some(800), Some(800), None, Some(800)]),
    checkpoint(CareerPhase::Senior, 12, [Some(1350), Some(1000), Some(1000), None, Some(1000)]),
];
