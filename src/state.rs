use crate::domain::{CareerPhase, FragmentToken, GoddessColor, TrainingType};
use crate::events::RunEvent;
use crate::tables;
use crate::tree::{self, PredictedTree, SPIRIT_FRAGMENT_MAX};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

/// One tracked facility.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FacilityInfo {
    pub clicks: u32,
    pub fragments: u32,
    pub points: u32,
    pub level: u32,
}

/// One recorded burst.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BurstRecord {
    pub turn: u32,
    pub color: GoddessColor,
    pub phase: CareerPhase,
}

/// Everything derived from the event list so far.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunSnapshot {
    pub phase: CareerPhase,
    pub last_turn: u32,
    pub pending_slots: Vec<FragmentToken>,
    pub tree: PredictedTree,
    pub facilities: [FacilityInfo; TrainingType::COUNT],
    pub goddess_levels: [u8; GoddessColor::COUNT],
    pub bursts: Vec<BurstRecord>,
    pub races_won: u32,
    pub races_lost: u32,
}

/// Rebuilds the full snapshot from the events; pass a seeded rng for identical replays.
pub fn replay(events: &[RunEvent], rng: &mut impl Rng) -> RunSnapshot {
    let mut phase = CareerPhase::Junior;
    let mut last_turn = 0;
    let mut pending: Vec<FragmentToken> = Vec::new();
    let mut clicks = [0u32; TrainingType::COUNT];
    let mut fragments = [0u32; TrainingType::COUNT];
    let mut races_won = 0;
    let mut races_lost = 0;
    let mut bursts: Vec<BurstRecord> = Vec::new();

    for event in events {
        match event {
            RunEvent::PhaseStarted { turn, phase: new_phase } => {
                last_turn = *turn;
                phase = *new_phase;
            }
            RunEvent::TrainingClick { turn, training } => {
                last_turn = *turn;
                clicks[training.index()] += 1;
            }
            RunEvent::FragmentGained { turn, color, training, source } => {
                last_turn = *turn;
                fragments[training.index()] += 1;
                pending.push(FragmentToken {
                    color: *color,
                    training: *training,
                    source: *source,
                    turn: *turn,
                });
            }
            RunEvent::GmRaceCompleted { turn, won } => {
                last_turn = *turn;
                if *won {
                    races_won += 1;
                } else {
                    races_lost += 1;
                }
            }
            RunEvent::BurstActivated { turn, color } => {
                last_turn = *turn;
                bursts.push(BurstRecord { turn: *turn, color: *color, phase });
                let consumed = pending.len().min(SPIRIT_FRAGMENT_MAX as usize);
                pending.drain(..consumed);
            }
        }
    }

    let race_points =
        races_won * tables::RACE_WIN_POINTS + races_lost * tables::RACE_LOSS_POINTS;
    let facilities: [FacilityInfo; TrainingType::COUNT] = std::array::from_fn(|i| {
        let clicks = clicks[i];
        let fragments = fragments[i];
        let points = clicks * tables::POINTS_PER_TRAINING_CLICK
            + fragments * tables::POINTS_PER_FRAGMENT
            + fragments / tables::FRAGMENTS_PER_EXTRA_POINT
            + race_points;
        FacilityInfo {
            clicks,
            fragments,
            points,
            level: points / tables::POINTS_PER_FACILITY_LEVEL,
        }
    });

    let mut goddess_levels = [0u8; GoddessColor::COUNT];
    for color in GoddessColor::ALL {
        let count = bursts.iter().filter(|burst| burst.color == color).count();
        goddess_levels[color.index()] = count.min(5) as u8;
    }

    RunSnapshot {
        phase,
        last_turn,
        pending_slots: pending.clone(),
        tree: tree::predict(&pending, rng),
        facilities,
        goddess_levels,
        bursts,
        races_won,
        races_lost,
    }
}

/// [`replay`] with a fixed seed.
pub fn replay_seeded(events: &[RunEvent], seed: u64) -> RunSnapshot {
    let mut rng = StdRng::seed_from_u64(seed);
    replay(events, &mut rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FragmentSource, TrainingType};
    use crate::events::RunEvent;

    fn fragment(turn: u32, color: GoddessColor, training: TrainingType) -> RunEvent {
        RunEvent::FragmentGained {
            turn,
            color,
            training,
            source: FragmentSource::Training,
        }
    }

    #[test]
    fn facility_points_from_clicks_fragments_and_races() {
        let events = vec![
            RunEvent::TrainingClick { turn: 1, training: TrainingType::Speed },
            RunEvent::TrainingClick { turn: 2, training: TrainingType::Speed },
            RunEvent::TrainingClick { turn: 3, training: TrainingType::Speed },
            fragment(1, GoddessColor::Blue, TrainingType::Speed),
            fragment(2, GoddessColor::Blue, TrainingType::Speed),
            fragment(3, GoddessColor::Blue, TrainingType::Speed),
            fragment(4, GoddessColor::Blue, TrainingType::Speed),
            fragment(5, GoddessColor::Blue, TrainingType::Speed),
            RunEvent::GmRaceCompleted { turn: 6, won: true },
        ];
        let snapshot = replay_seeded(&events, 0);

        let speed = &snapshot.facilities[TrainingType::Speed.index()];
        assert_eq!(speed.points, 20);
        assert_eq!(speed.level, 1);

        let stamina = &snapshot.facilities[TrainingType::Stamina.index()];
        assert_eq!(stamina.points, tables::RACE_WIN_POINTS);
    }

    #[test]
    fn burst_consumes_one_tree_and_keeps_overflow() {
        let mut events: Vec<RunEvent> = (1..=10)
            .map(|turn| fragment(turn, GoddessColor::Blue, TrainingType::Speed))
            .collect();
        events.push(RunEvent::BurstActivated { turn: 11, color: GoddessColor::Blue });

        let snapshot = replay_seeded(&events, 0);
        assert_eq!(snapshot.pending_slots.len(), 2);
        assert!(snapshot.tree.layer2[0].is_some());
        assert!(!snapshot.tree.is_complete());
    }

    #[test]
    fn burst_without_a_full_tree_consumes_what_is_there() {
        let events = vec![
            fragment(1, GoddessColor::Blue, TrainingType::Speed),
            RunEvent::BurstActivated { turn: 2, color: GoddessColor::Blue },
        ];
        let snapshot = replay_seeded(&events, 0);
        assert!(snapshot.pending_slots.is_empty());
    }

    #[test]
    fn goddess_levels_follow_burst_counts_and_cap_at_5() {
        let events: Vec<RunEvent> = (1..=6)
            .map(|turn| RunEvent::BurstActivated { turn, color: GoddessColor::Blue })
            .collect();
        let snapshot = replay_seeded(&events, 0);
        assert_eq!(snapshot.goddess_levels[GoddessColor::Blue.index()], 5);
        assert_eq!(snapshot.goddess_levels[GoddessColor::Red.index()], 0);
    }

    #[test]
    fn phase_start_updates_phase_and_burst_records_track_it() {
        let events = vec![
            RunEvent::BurstActivated { turn: 5, color: GoddessColor::Blue },
            RunEvent::PhaseStarted { turn: 24, phase: CareerPhase::Classic },
            RunEvent::BurstActivated { turn: 30, color: GoddessColor::Red },
        ];
        let snapshot = replay_seeded(&events, 0);
        assert_eq!(snapshot.phase, CareerPhase::Classic);
        assert_eq!(snapshot.bursts[0].phase, CareerPhase::Junior);
        assert_eq!(snapshot.bursts[1].phase, CareerPhase::Classic);
    }

    #[test]
    fn replay_is_deterministic_for_a_fixed_seed() {
        let mut events: Vec<RunEvent> = (1..=8)
            .map(|turn| fragment(turn, GoddessColor::Blue, TrainingType::Speed))
            .collect();
        events.push(RunEvent::BurstActivated { turn: 9, color: GoddessColor::Blue });

        let first = replay_seeded(&events, 12345);
        let second = replay_seeded(&events, 12345);
        assert_eq!(first, second);
    }
}
