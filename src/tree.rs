use crate::domain::{FragmentToken, GoddessColor, TrainingType};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Slot layout, mirroring `SingleModeScenarioVenusDefine`.
pub const SPIRIT_FRAGMENT_MAX: u8 = 8;
/// First slot past layer 2 (slots 8..=11 are layer 2).
pub const SPIRIT_LEVEL2_INDEX_MAX: u8 = 12;
/// First slot past layer 3 (slots 12..=13 are layer 3, 14 the top merge).
pub const SPIRIT_LEVEL3_INDEX_MAX: u8 = 14;
/// The absorbed goddess; the game's gate for activating a burst.
pub const VENUS_SPIRIT_INDEX: u8 = 15;
/// Slots 0..=15 in one array.
pub const SLOT_TOTAL: usize = 16;

/// The three goddesses as chara ids: 9040 red, 9041 blue, 9042 yellow.
pub const VENUS_CHARA_RED: u32 = 9040;
pub const VENUS_CHARA_BLUE: u32 = 9041;
pub const VENUS_CHARA_YELLOW: u32 = 9042;

/// A goddess chara id as the packet carries it, mapped to its color.
pub fn goddess_color(chara_id: u32) -> Option<GoddessColor> {
    match chara_id {
        VENUS_CHARA_RED => Some(GoddessColor::Red),
        VENUS_CHARA_BLUE => Some(GoddessColor::Blue),
        VENUS_CHARA_YELLOW => Some(GoddessColor::Yellow),
        _ => None,
    }
}

/// One `SingleModeVenusSpiritInfo` row: a slot occupancy as the packet reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpiritSlot {
    /// Position 0..=15; slot 15 carries the goddess chara_id in `spirit_id`.
    pub spirit_num: u8,
    /// Fragment id, or the goddess chara_id when `spirit_num == 15`.
    pub spirit_id: u32,
    /// Resolved effect group for that placement.
    pub effect_group_id: u32,
}

/// The observed tree, indexed from a packet's `spirit_info_array`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct TreeSnapshot {
    slots: [Option<SpiritSlot>; SLOT_TOTAL],
}

impl TreeSnapshot {
    /// Indexes the packet rows by slot; slot numbers past 15 are dropped.
    pub fn from_packet(slots: &[SpiritSlot]) -> TreeSnapshot {
        let mut tree = TreeSnapshot::default();
        for slot in slots {
            if (slot.spirit_num as usize) < SLOT_TOTAL {
                tree.slots[slot.spirit_num as usize] = Some(*slot);
            }
        }
        tree
    }

    pub fn slot(&self, spirit_num: u8) -> Option<SpiritSlot> {
        self.slots.get(spirit_num as usize).copied().flatten()
    }

    /// Occupied fragment slots (0..=7), in slot order.
    pub fn fragment_slots(&self) -> Vec<SpiritSlot> {
        self.slots[..SPIRIT_FRAGMENT_MAX as usize]
            .iter()
            .copied()
            .flatten()
            .collect()
    }

    /// The game's absorb gate: a goddess sits in slot 15.
    pub fn is_ready_to_absorb(&self) -> bool {
        self.slots[VENUS_SPIRIT_INDEX as usize].is_some()
    }

    /// The color of the goddess sitting in slot 15.
    pub fn burst_color(&self) -> Option<GoddessColor> {
        self.slots[VENUS_SPIRIT_INDEX as usize]
            .and_then(|slot| goddess_color(slot.spirit_id))
    }

    /// How many bottom slots are filled; the index the next fragment takes.
    pub fn next_slot_index(&self) -> usize {
        self.fragment_slots().len()
    }
}

/// A predicted merge, with the bottom fragments backing it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TreeNode {
    pub color: GoddessColor,
    pub training: TrainingType,
    pub stat_bonus: u8,
    pub backing: Vec<FragmentToken>,
}

/// Slots 8..=15 as a full bottom row would produce them.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PredictedTree {
    /// Slots 8..=11, each merging one adjacent fragment pair.
    pub layer2: [Option<TreeNode>; 4],
    /// Slots 12..=13, each merging one layer-2 pair.
    pub layer3: [Option<TreeNode>; 2],
    /// Slot 14, the merge of the two layer-3 nodes.
    pub top: Option<TreeNode>,
    /// Slot 15, the absorb: the PK winner of the layer-3 pair.
    pub goddess: Option<TreeNode>,
}

impl PredictedTree {
    /// True when the bottom row is full and the absorb color is settled.
    pub fn is_complete(&self) -> bool {
        self.goddess.is_some()
    }

    /// The predicted burst color.
    pub fn burst_color(&self) -> Option<GoddessColor> {
        self.goddess.as_ref().map(|node| node.color)
    }
}

/// Predicts slots 8..=15 from the bottom fragments; a seeded rng makes replays identical.
pub fn predict(bottom: &[FragmentToken], rng: &mut impl Rng) -> PredictedTree {
    let mut coin = || rng.random_bool(0.8);
    predict_with(bottom, &mut coin)
}

/// Same as [`predict`], but the "left wins" coin (training inheritance, PK ties) is explicit.
pub fn predict_with(bottom: &[FragmentToken], coin: &mut dyn FnMut() -> bool) -> PredictedTree {
    let fragments: Vec<FragmentToken> = bottom
        .iter()
        .take(SPIRIT_FRAGMENT_MAX as usize)
        .cloned()
        .collect();
    let mut tree = PredictedTree::default();

    let mut index = 0;
    for pair in fragments.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        tree.layer2[index] = Some(merge_pair(&pair[0], &pair[1], coin));
        index += 1;
    }

    if let (Some(left), Some(right)) = (&tree.layer2[0], &tree.layer2[1]) {
        tree.layer3[0] = Some(merge_nodes(left, right, coin));
    }
    if let (Some(left), Some(right)) = (&tree.layer2[2], &tree.layer2[3]) {
        tree.layer3[1] = Some(merge_nodes(left, right, coin));
    }
    if let (Some(left), Some(right)) = (&tree.layer3[0], &tree.layer3[1]) {
        tree.top = Some(merge_nodes(left, right, coin));
        tree.goddess = Some(pk_top(left, right, coin));
    }
    tree
}

fn merge_pair(
    left: &FragmentToken,
    right: &FragmentToken,
    coin: &mut dyn FnMut() -> bool,
) -> TreeNode {
    TreeNode {
        color: left.color,
        training: if coin() { left.training } else { right.training },
        stat_bonus: if left.color == right.color { 2 } else { 3 },
        backing: vec![left.clone(), right.clone()],
    }
}

fn merge_nodes(left: &TreeNode, right: &TreeNode, coin: &mut dyn FnMut() -> bool) -> TreeNode {
    let mut backing = left.backing.clone();
    backing.extend_from_slice(&right.backing);
    TreeNode {
        color: left.color,
        training: if coin() { left.training } else { right.training },
        stat_bonus: if left.color == right.color { 2 } else { 3 },
        backing,
    }
}

/// The absorb "PK": the layer-3 node backed by more bottom fragments of its
/// own color wins; a tie falls to the coin.
fn pk_top(left: &TreeNode, right: &TreeNode, coin: &mut dyn FnMut() -> bool) -> TreeNode {
    let left_count = left.backing.iter().filter(|f| f.color == left.color).count();
    let right_count = right
        .backing
        .iter()
        .filter(|f| f.color == right.color)
        .count();
    let color = match left_count.cmp(&right_count) {
        Ordering::Greater => left.color,
        Ordering::Less => right.color,
        Ordering::Equal => {
            if coin() {
                left.color
            } else {
                right.color
            }
        }
    };
    let mut backing = left.backing.clone();
    backing.extend_from_slice(&right.backing);
    TreeNode {
        color,
        training: if coin() { left.training } else { right.training },
        stat_bonus: if left.color == right.color { 2 } else { 3 },
        backing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FragmentSource, GoddessColor, TrainingType};

    fn token(color: GoddessColor, training: TrainingType) -> FragmentToken {
        FragmentToken {
            color,
            training,
            source: FragmentSource::Training,
            turn: 1,
        }
    }

    fn full_row(colors: [GoddessColor; 8]) -> Vec<FragmentToken> {
        colors.map(|c| token(c, TrainingType::Speed)).to_vec()
    }

    #[test]
    fn layer2_color_follows_the_left_fragment() {
        let row = vec![
            token(GoddessColor::Red, TrainingType::Speed),
            token(GoddessColor::Blue, TrainingType::Stamina),
        ];
        let tree = predict_with(&row, &mut || true);
        assert_eq!(tree.layer2[0].as_ref().unwrap().color, GoddessColor::Red);
        assert_eq!(tree.layer2[0].as_ref().unwrap().stat_bonus, 3);
    }

    #[test]
    fn matching_colors_grant_bonus_2() {
        let row = vec![
            token(GoddessColor::Red, TrainingType::Speed),
            token(GoddessColor::Red, TrainingType::Stamina),
        ];
        let tree = predict_with(&row, &mut || true);
        assert_eq!(tree.layer2[0].as_ref().unwrap().color, GoddessColor::Red);
        assert_eq!(tree.layer2[0].as_ref().unwrap().stat_bonus, 2);
    }

    #[test]
    fn training_inheritance_follows_the_coin() {
        let row = vec![
            token(GoddessColor::Red, TrainingType::Speed),
            token(GoddessColor::Blue, TrainingType::Stamina),
        ];

        let left_wins = predict_with(&row, &mut || true);
        assert_eq!(
            left_wins.layer2[0].as_ref().unwrap().training,
            TrainingType::Speed
        );

        let right_wins = predict_with(&row, &mut || false);
        assert_eq!(
            right_wins.layer2[0].as_ref().unwrap().training,
            TrainingType::Stamina
        );
    }

    #[test]
    fn partial_rows_merge_only_complete_pairs() {
        let row = full_row([GoddessColor::Blue; 8])[..5].to_vec();
        let tree = predict_with(&row, &mut || true);
        assert!(tree.layer2[0].is_some());
        assert!(tree.layer2[1].is_some());
        assert!(tree.layer2[2].is_none());
        assert!(tree.layer3[0].is_some());
        assert!(tree.layer3[1].is_none());
        assert!(tree.top.is_none());
        assert!(!tree.is_complete());
    }

    #[test]
    fn absorb_pk_favors_more_same_color_backing() {
        let mut row = full_row([
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Blue,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
        ]);
        row[1].training = TrainingType::Stamina;
        let tree = predict_with(&row, &mut || true);
        assert_eq!(tree.burst_color(), Some(GoddessColor::Yellow));
        assert!(tree.is_complete());
    }

    #[test]
    fn absorb_pk_tie_breaks_with_the_coin() {
        let row = full_row([
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
        ]);

        let left_wins = predict_with(&row, &mut || true);
        assert_eq!(left_wins.burst_color(), Some(GoddessColor::Red));

        let right_wins = predict_with(&row, &mut || false);
        assert_eq!(right_wins.burst_color(), Some(GoddessColor::Yellow));
    }

    #[test]
    fn observed_packet_slots_index_by_number() {
        let slots = vec![
            SpiritSlot { spirit_num: 2, spirit_id: 501, effect_group_id: 10 },
            SpiritSlot { spirit_num: 8, spirit_id: 502, effect_group_id: 11 },
            SpiritSlot { spirit_num: 15, spirit_id: VENUS_CHARA_YELLOW, effect_group_id: 12 },
        ];
        let tree = TreeSnapshot::from_packet(&slots);
        assert_eq!(tree.fragment_slots().len(), 1);
        assert_eq!(tree.next_slot_index(), 1);
        assert_eq!(tree.slot(8).unwrap().spirit_id, 502);
        assert!(tree.is_ready_to_absorb());
        assert_eq!(tree.burst_color(), Some(GoddessColor::Yellow));
    }

    #[test]
    fn goddess_chara_ids_map_to_colors() {
        assert_eq!(goddess_color(VENUS_CHARA_RED), Some(GoddessColor::Red));
        assert_eq!(goddess_color(VENUS_CHARA_BLUE), Some(GoddessColor::Blue));
        assert_eq!(goddess_color(VENUS_CHARA_YELLOW), Some(GoddessColor::Yellow));
        assert_eq!(goddess_color(0), None);
    }
}
