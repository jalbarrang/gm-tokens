use crate::domain::{FragmentSource, GoddessColor, TrainingType};
use crate::events::RunEvent;
use crate::tree::{goddess_color, SpiritSlot, SPIRIT_FRAGMENT_MAX};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// One `SingleModeVenusActiveSpiritEffect` row: a burst in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSpiritEffect {
    pub chara_id: u32,
    pub effect_group_id: u32,
    pub begin_turn: u32,
    pub end_turn: u32,
}

/// One `SingleModeVenusCharaInfo` row: a goddess's insight level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenusCharaInfo {
    pub chara_id: u32,
    pub venus_level: u32,
}

/// One `SingleModeVenusRaceHistory` row: a scenario race already run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenusRaceHistory {
    pub race_num: u32,
    pub turn: u32,
    pub result_rank: u32,
}

/// The four `SingleModeVenusDataSet` arrays the tracker diffs, with the
/// packet's own field names. The other eight keys of the map are unused here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VenusDataSet {
    #[serde(default)]
    pub spirit_info_array: Vec<SpiritSlot>,
    #[serde(default)]
    pub venus_spirit_active_effect_info_array: Vec<ActiveSpiritEffect>,
    #[serde(default)]
    pub venus_chara_info_array: Vec<VenusCharaInfo>,
    #[serde(default)]
    pub venus_race_history_array: Vec<VenusRaceHistory>,
}

/// The master-data joins the packet cannot carry.
#[derive(Debug, Clone, Default)]
pub struct MasterData {
    /// spirit_id -> owning goddess chara_id (`single_mode_venus_spirit_group.CharaId`).
    pub spirit_color: HashMap<u32, u32>,
    /// effect_group_id -> its ParamUp attribute (`single_mode_venus_spirit_effect`, EffectType 1).
    pub effect_attribute: HashMap<u32, TrainingType>,
}

impl MasterData {
    /// A slot's fragment identity, or None if the master rows are missing.
    pub fn resolve(&self, slot: &SpiritSlot) -> Option<(GoddessColor, TrainingType)> {
        let color = goddess_color(*self.spirit_color.get(&slot.spirit_id)?)?;
        let training = self.effect_attribute.get(&slot.effect_group_id).copied()?;
        Some((color, training))
    }
}

/// Diffs two consecutive packets into events, mirroring the game's own delta
/// logic (`WorkSingleModeChangeParameterInfo`): a new bottom slot is a
/// fragment, a grown active-effect array is a burst, a new race-history row
/// is a GM race. `turn` stamps fragment events; `source` says which action
/// produced this response, since the packet does not carry either.
pub fn events_between(
    prev: &VenusDataSet,
    next: &VenusDataSet,
    masters: &MasterData,
    turn: u32,
    source: FragmentSource,
) -> Vec<RunEvent> {
    let mut events = Vec::new();

    let prev_bottom: HashSet<u8> = prev
        .spirit_info_array
        .iter()
        .map(|slot| slot.spirit_num)
        .filter(|num| *num < SPIRIT_FRAGMENT_MAX)
        .collect();
    for slot in &next.spirit_info_array {
        if slot.spirit_num >= SPIRIT_FRAGMENT_MAX || prev_bottom.contains(&slot.spirit_num) {
            continue;
        }
        if let Some((color, training)) = masters.resolve(slot) {
            events.push(RunEvent::FragmentGained {
                turn,
                color,
                training,
                source,
            });
        }
    }

    if next.venus_spirit_active_effect_info_array.len()
        > prev.venus_spirit_active_effect_info_array.len()
    {
        let newest = next
            .venus_spirit_active_effect_info_array
            .iter()
            .max_by_key(|effect| effect.begin_turn);
        if let Some(effect) = newest {
            if let Some(color) = goddess_color(effect.chara_id) {
                events.push(RunEvent::BurstActivated {
                    turn: effect.begin_turn,
                    color,
                });
            }
        }
    }

    let prev_races: HashSet<u32> = prev
        .venus_race_history_array
        .iter()
        .map(|race| race.race_num)
        .collect();
    for race in &next.venus_race_history_array {
        if prev_races.contains(&race.race_num) {
            continue;
        }
        events.push(RunEvent::GmRaceCompleted {
            turn: race.turn,
            won: race.result_rank == 1,
        });
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{GoddessColor, TrainingType};
    use crate::tree::VENUS_CHARA_BLUE;

    fn masters() -> MasterData {
        MasterData {
            spirit_color: HashMap::from([(501, 9040), (502, 9041)]),
            effect_attribute: HashMap::from([
                (10, TrainingType::Speed),
                (11, TrainingType::Wit),
            ]),
        }
    }

    #[test]
    fn new_bottom_slots_become_fragment_events() {
        let prev = VenusDataSet {
            spirit_info_array: vec![SpiritSlot { spirit_num: 0, spirit_id: 501, effect_group_id: 10 }],
            ..VenusDataSet::default()
        };
        let next = VenusDataSet {
            spirit_info_array: vec![
                SpiritSlot { spirit_num: 0, spirit_id: 501, effect_group_id: 10 },
                SpiritSlot { spirit_num: 1, spirit_id: 502, effect_group_id: 11 },
                SpiritSlot { spirit_num: 9, spirit_id: 502, effect_group_id: 11 },
            ],
            ..VenusDataSet::default()
        };

        let events = events_between(&prev, &next, &masters(), 12, FragmentSource::Training);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            RunEvent::FragmentGained {
                turn: 12,
                color: GoddessColor::Blue,
                training: TrainingType::Wit,
                source: FragmentSource::Training,
            }
        );
    }

    #[test]
    fn a_grown_active_effect_array_is_a_burst() {
        let next = VenusDataSet {
            venus_spirit_active_effect_info_array: vec![ActiveSpiritEffect {
                chara_id: VENUS_CHARA_BLUE,
                effect_group_id: 11,
                begin_turn: 40,
                end_turn: 42,
            }],
            ..VenusDataSet::default()
        };

        let events = events_between(
            &VenusDataSet::default(),
            &next,
            &masters(),
            40,
            FragmentSource::GoddessEvent,
        );
        assert_eq!(
            events,
            vec![RunEvent::BurstActivated { turn: 40, color: GoddessColor::Blue }]
        );
    }

    #[test]
    fn new_race_history_rows_become_race_events() {
        let prev = VenusDataSet {
            venus_race_history_array: vec![VenusRaceHistory { race_num: 2, turn: 24, result_rank: 1 }],
            ..VenusDataSet::default()
        };
        let next = VenusDataSet {
            venus_race_history_array: vec![
                VenusRaceHistory { race_num: 2, turn: 24, result_rank: 1 },
                VenusRaceHistory { race_num: 3, turn: 48, result_rank: 3 },
            ],
            ..VenusDataSet::default()
        };

        let events = events_between(
            &prev,
            &next,
            &masters(),
            48,
            FragmentSource::TargetRace,
        );
        assert_eq!(events, vec![RunEvent::GmRaceCompleted { turn: 48, won: false }]);
    }
}
