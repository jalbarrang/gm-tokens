//! End-to-end scripted runs: the shape of a real junior year fed through the
//! event stream, checked against the guide's arithmetic.

use gm_tokens::domain::{CareerPhase, FragmentSource, GoddessColor, TrainingType};
use gm_tokens::events::RunEvent;
use gm_tokens::{advisories, state, tables};

fn fragment(turn: u32, color: GoddessColor, training: TrainingType) -> RunEvent {
    RunEvent::FragmentGained { turn, color, training, source: FragmentSource::Training }
}

#[test]
fn scripted_junior_year() {
    // A simplified junior year: build bonds and train speed, win the GM
    // race, and land the first burst as Blue.
    let mut events = vec![RunEvent::PhaseStarted { turn: 1, phase: CareerPhase::Junior }];

    // Six training turns, one fragment each (no rainbows yet).
    for turn in 1..=6 {
        events.push(RunEvent::TrainingClick { turn, training: TrainingType::Speed });
        events.push(fragment(turn, GoddessColor::Blue, TrainingType::Speed));
    }
    // The GM race: a win feeds every facility and drops one fragment of each color.
    events.push(RunEvent::GmRaceCompleted { turn: 7, won: true });
    events.push(RunEvent::FragmentGained {
        turn: 7,
        color: GoddessColor::Red,
        training: TrainingType::Power,
        source: FragmentSource::TargetRace,
    });
    events.push(RunEvent::FragmentGained {
        turn: 7,
        color: GoddessColor::Blue,
        training: TrainingType::Speed,
        source: FragmentSource::TargetRace,
    });
    events.push(RunEvent::FragmentGained {
        turn: 7,
        color: GoddessColor::Yellow,
        training: TrainingType::Wit,
        source: FragmentSource::TargetRace,
    });
    // Two more fragments fill the tree (6 + 3 raced in = 9, one overflow).
    events.push(fragment(8, GoddessColor::Blue, TrainingType::Speed));
    events.push(fragment(9, GoddessColor::Blue, TrainingType::Speed));
    // Activate the first burst as Blue.
    events.push(RunEvent::BurstActivated { turn: 10, color: GoddessColor::Blue });

    let snapshot = state::replay_seeded(&events, 42);

    // The tree consumed 8 of the 11 fragments; three carry over.
    assert_eq!(snapshot.pending_slots.len(), 3);
    assert_eq!(snapshot.tree.next_slot_index(), 3);
    assert!(!snapshot.tree.is_complete());

    // Goddess levels: Blue 1, others 0.
    assert_eq!(snapshot.goddess_levels[GoddessColor::Blue.index()], 1);
    assert_eq!(snapshot.goddess_levels[GoddessColor::Red.index()], 0);

    // Speed facility: 6 clicks * 2 + 9 speed fragments (8 + 1 per-5) + 8 race
    // points = 30.
    let speed = &snapshot.facilities[TrainingType::Speed.index()];
    assert_eq!(speed.points, 30);
    assert_eq!(speed.level, 2);

    // Burst plan progress shows; with slot 1 blue and a burst already
    // recorded, neither the overcap nor the first-burst advisory fires.
    let text: Vec<String> = advisories::evaluate(&snapshot)
        .into_iter()
        .map(|a| a.message)
        .collect();
    assert!(text.iter().any(|m| m.contains("1/3 bursts recorded in Junior")));
    assert!(!text.iter().any(|m| m.contains("overcapping")));
    assert!(!text.iter().any(|m| m.contains("First burst of the run should be Blue")));
}

#[test]
fn the_guide_tables_survive_a_round_trip() {
    // The tables are plain data; this pins the numbers the guide states so a
    // careless edit fails loudly instead of silently drifting.
    assert_eq!(tables::expected_bursts(CareerPhase::Junior), 3);
    assert_eq!(tables::expected_bursts(CareerPhase::Classic), 4);
    assert_eq!(tables::expected_bursts(CareerPhase::Senior), 4);
    assert_eq!(tables::expected_bursts(CareerPhase::UraFinale), 1);
    assert_eq!(tables::burst_value_estimate(GoddessColor::Red), (90, 100));
    assert_eq!(tables::POINTS_PER_FACILITY_LEVEL, 12);
    assert_eq!(tables::ATTRIBUTE_DROP_RATES[TrainingType::SkillPoints.index()], 0.241);
    assert_eq!(tables::STAT_CHECKPOINTS.len(), 4);
}
