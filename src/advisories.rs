use crate::domain::{CareerPhase, GoddessColor};
use crate::state::RunSnapshot;
use crate::tables;
use serde::Serialize;

/// Advisory severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AdvisoryKind {
    Info,
    Warning,
}

/// One piece of advice; hosts decide how to display it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Advisory {
    pub kind: AdvisoryKind,
    pub message: String,
}

fn info(message: impl Into<String>) -> Advisory {
    Advisory { kind: AdvisoryKind::Info, message: message.into() }
}

fn warning(message: impl Into<String>) -> Advisory {
    Advisory { kind: AdvisoryKind::Warning, message: message.into() }
}

/// The guide's rules of thumb for this snapshot, as plain data.
pub fn evaluate(snapshot: &RunSnapshot) -> Vec<Advisory> {
    let mut out: Vec<Advisory> = Vec::new();

    let next_slot = snapshot.pending_slots.len();
    if next_slot == 0 || next_slot == 4 {
        let slot = if next_slot == 0 { 1 } else { 5 };
        out.push(info(format!(
            "Slot {slot} opens next: golden-rule slot — lock your target color, and avoid dropping below 50 energy on this turn."
        )));
    }

    if snapshot
        .pending_slots
        .first()
        .is_some_and(|f| f.color == GoddessColor::Red)
    {
        out.push(warning(
            "Slot 1 is red: a Red burst grants +50 energy — watch for overcapping.",
        ));
    }

    if snapshot.bursts.is_empty() {
        out.push(info(
            "First burst of the run should be Blue (bond velocity + hint-rate passive).",
        ));
    }

    let yellows = snapshot
        .bursts
        .iter()
        .filter(|burst| burst.color == GoddessColor::Yellow)
        .count();
    if yellows > tables::YELLOW_BURST_CAP as usize {
        out.push(warning(format!(
            "Yellow burst #{yellows}: cap is {} per run ({} tolerated).",
            tables::YELLOW_BURST_CAP,
            tables::YELLOW_BURST_CAP + 1
        )));
    }

    let expected = tables::expected_bursts(snapshot.phase);
    let actual = snapshot
        .bursts
        .iter()
        .filter(|burst| burst.phase == snapshot.phase)
        .count() as u8;
    out.push(info(format!(
        "{actual}/{expected} bursts recorded in {:?}.",
        snapshot.phase
    )));

    let reds_in_phase = snapshot
        .bursts
        .iter()
        .any(|burst| burst.phase == snapshot.phase && burst.color == GoddessColor::Red);
    match snapshot.phase {
        CareerPhase::UraFinale if !reds_in_phase => out.push(warning(
            "Force a Red burst in the URA finale — Blue and Yellow give zero value on the final race.",
        )),
        CareerPhase::Classic | CareerPhase::Senior if !reds_in_phase => {
            out.push(info("Plan the year-end burst as Red."))
        }
        _ => {}
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CareerPhase, FragmentSource, GoddessColor, TrainingType};
    use crate::events::RunEvent;
    use crate::state;

    fn snapshot(events: &[RunEvent]) -> RunSnapshot {
        state::replay_seeded(events, 0)
    }

    fn messages(advisories: &[Advisory]) -> Vec<&str> {
        advisories.iter().map(|a| a.message.as_str()).collect()
    }

    #[test]
    fn golden_rule_slots_are_flagged() {
        let empty = snapshot(&[]);
        assert!(messages(&evaluate(&empty)).iter().any(|m| m.contains("Slot 1")));

        let mut events = vec![RunEvent::FragmentGained {
            turn: 1,
            color: GoddessColor::Blue,
            training: TrainingType::Speed,
            source: FragmentSource::Training,
        }];
        events.extend((2..=4).map(|turn| RunEvent::FragmentGained {
            turn,
            color: GoddessColor::Blue,
            training: TrainingType::Speed,
            source: FragmentSource::Training,
        }));
        let four = snapshot(&events);
        assert!(messages(&evaluate(&four)).iter().any(|m| m.contains("Slot 5")));
    }

    #[test]
    fn red_in_slot_1_warns_about_energy() {
        let events = vec![RunEvent::FragmentGained {
            turn: 1,
            color: GoddessColor::Red,
            training: TrainingType::Speed,
            source: FragmentSource::Training,
        }];
        let snapshot = snapshot(&events);
        assert!(messages(&evaluate(&snapshot))
            .iter()
            .any(|m| m.contains("overcapping")));
    }

    #[test]
    fn yellow_over_cap_warns() {
        let events: Vec<RunEvent> = (1..=4)
            .map(|turn| RunEvent::BurstActivated { turn, color: GoddessColor::Yellow })
            .collect();
        let snapshot = snapshot(&events);
        assert!(messages(&evaluate(&snapshot))
            .iter()
            .any(|m| m.contains("Yellow burst #4")));
    }

    #[test]
    fn ura_finale_demands_red() {
        let events = vec![
            RunEvent::PhaseStarted { turn: 70, phase: CareerPhase::UraFinale },
            RunEvent::BurstActivated { turn: 71, color: GoddessColor::Blue },
        ];
        let snapshot = snapshot(&events);
        assert!(messages(&evaluate(&snapshot))
            .iter()
            .any(|m| m.contains("URA finale")));
    }
}
