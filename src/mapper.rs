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
#[derive(Debug, Clone, Default, Deserialize)]
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

/// One packet diff: the events it produced and what it could not resolve.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PacketDiff {
    pub events: Vec<RunEvent>,
    /// New bottom slots whose master rows were missing; their fragments were skipped.
    pub unresolved_slots: usize,
}

/// Diffs two consecutive packets into events, mirroring the game's own delta
/// logic (`WorkSingleModeChangeParameterInfo`): a new bottom slot is a
/// fragment, a new active-effect row is a burst, a new race-history row is a
/// GM race. `turn` stamps fragment events; `source` says which action
/// produced this response, since the packet does not carry either.
///
/// `prev` and `next` must be consecutive packets of one live session. To
/// start tracking mid-run, seed `prev` with the first packet observed —
/// diffing it against itself yields nothing, while a fresh default would
/// count every placed fragment as newly gained.
pub fn events_between(
    prev: &VenusDataSet,
    next: &VenusDataSet,
    masters: &MasterData,
    turn: u32,
    source: FragmentSource,
) -> PacketDiff {
    let mut events = Vec::new();
    let mut unresolved_slots = 0;

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
        match masters.resolve(slot) {
            Some((color, training)) => events.push(RunEvent::FragmentGained {
                turn,
                color,
                training,
                source,
            }),
            None => unresolved_slots += 1,
        }
    }

    let prev_effects: HashSet<(u32, u32)> = prev
        .venus_spirit_active_effect_info_array
        .iter()
        .map(|effect| (effect.chara_id, effect.begin_turn))
        .collect();
    for effect in &next.venus_spirit_active_effect_info_array {
        if prev_effects.contains(&(effect.chara_id, effect.begin_turn)) {
            continue;
        }
        if let Some(color) = goddess_color(effect.chara_id) {
            events.push(RunEvent::BurstActivated {
                turn: effect.begin_turn,
                color,
            });
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

    PacketDiff {
        events,
        unresolved_slots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{GoddessColor, TrainingType};
    use crate::tree::{VENUS_CHARA_BLUE, VENUS_CHARA_RED, VENUS_CHARA_YELLOW};

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

        let diff = events_between(&prev, &next, &masters(), 12, FragmentSource::Training);
        assert_eq!(
            diff,
            PacketDiff {
                events: vec![RunEvent::FragmentGained {
                    turn: 12,
                    color: GoddessColor::Blue,
                    training: TrainingType::Wit,
                    source: FragmentSource::Training,
                }],
                unresolved_slots: 0,
            }
        );
    }

    #[test]
    fn each_new_active_effect_row_is_a_burst() {
        let next = VenusDataSet {
            venus_spirit_active_effect_info_array: vec![
                ActiveSpiritEffect {
                    chara_id: VENUS_CHARA_BLUE,
                    effect_group_id: 11,
                    begin_turn: 40,
                    end_turn: 42,
                },
                ActiveSpiritEffect {
                    chara_id: VENUS_CHARA_RED,
                    effect_group_id: 10,
                    begin_turn: 41,
                    end_turn: 43,
                },
                ActiveSpiritEffect {
                    chara_id: VENUS_CHARA_YELLOW,
                    effect_group_id: 10,
                    begin_turn: 30,
                    end_turn: 32,
                },
            ],
            ..VenusDataSet::default()
        };

        let diff = events_between(
            &VenusDataSet::default(),
            &next,
            &masters(),
            40,
            FragmentSource::GoddessEvent,
        );
        assert_eq!(
            diff.events,
            vec![
                RunEvent::BurstActivated { turn: 40, color: GoddessColor::Blue },
                RunEvent::BurstActivated { turn: 41, color: GoddessColor::Red },
                RunEvent::BurstActivated { turn: 30, color: GoddessColor::Yellow },
            ]
        );
    }

    #[test]
    fn diffing_a_packet_against_itself_yields_nothing() {
        let loaded = VenusDataSet {
            spirit_info_array: vec![
                SpiritSlot { spirit_num: 0, spirit_id: 501, effect_group_id: 10 },
                SpiritSlot { spirit_num: 8, spirit_id: 502, effect_group_id: 11 },
                SpiritSlot { spirit_num: 15, spirit_id: VENUS_CHARA_YELLOW, effect_group_id: 12 },
            ],
            venus_spirit_active_effect_info_array: vec![ActiveSpiritEffect {
                chara_id: VENUS_CHARA_BLUE,
                effect_group_id: 11,
                begin_turn: 40,
                end_turn: 42,
            }],
            venus_race_history_array: vec![VenusRaceHistory { race_num: 2, turn: 24, result_rank: 1 }],
            ..VenusDataSet::default()
        };

        let diff = events_between(&loaded, &loaded, &masters(), 42, FragmentSource::Training);
        assert_eq!(diff, PacketDiff::default());
    }

    #[test]
    fn unresolvable_slots_are_reported_not_dropped() {
        let next = VenusDataSet {
            spirit_info_array: vec![SpiritSlot { spirit_num: 0, spirit_id: 999, effect_group_id: 10 }],
            ..VenusDataSet::default()
        };

        let diff = events_between(
            &VenusDataSet::default(),
            &next,
            &masters(),
            12,
            FragmentSource::Training,
        );
        assert_eq!(
            diff,
            PacketDiff {
                events: Vec::new(),
                unresolved_slots: 1,
            }
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

        let diff = events_between(&prev, &next, &masters(), 48, FragmentSource::TargetRace);
        assert_eq!(
            diff.events,
            vec![RunEvent::GmRaceCompleted { turn: 48, won: false }]
        );
    }
}
