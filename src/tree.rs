use crate::domain::{FragmentToken, GoddessColor, TrainingType};
use rand::Rng;
use serde::Serialize;
use std::cmp::Ordering;

pub const SLOT_COUNT: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TreeNode {
    pub color: GoddessColor,
    pub training: TrainingType,
    pub stat_bonus: u8,
    pub backing: Vec<FragmentToken>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct TreeSnapshot {
    pub bottom_slots: Vec<FragmentToken>,
    pub layer1: Vec<TreeNode>,
    pub layer2: Vec<TreeNode>,
    pub top: Option<TreeNode>,
}

impl TreeSnapshot {
    pub fn is_complete(&self) -> bool {
        self.bottom_slots.len() >= SLOT_COUNT
    }

    pub fn burst_color(&self) -> Option<GoddessColor> {
        self.top.as_ref().map(|node| node.color)
    }

    pub fn next_slot_index(&self) -> usize {
        self.bottom_slots.len().min(SLOT_COUNT)
    }
}

pub fn build(slots: &[FragmentToken], rng: &mut impl Rng) -> TreeSnapshot {
    let mut coin = || rng.random_bool(0.8);
    build_with(slots, &mut coin)
}

pub fn build_with(slots: &[FragmentToken], coin: &mut dyn FnMut() -> bool) -> TreeSnapshot {
    let bottom: Vec<FragmentToken> = slots.iter().take(SLOT_COUNT).cloned().collect();
    let mut tree = TreeSnapshot {
        bottom_slots: bottom.clone(),
        ..TreeSnapshot::default()
    };

    for pair in bottom.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        tree.layer1.push(merge_pair(&pair[0], &pair[1], coin));
    }
    if tree.layer1.len() < 4 {
        return tree;
    }
    for pair in tree.layer1.chunks(2) {
        tree.layer2.push(merge_nodes(&pair[0], &pair[1], coin));
    }
    if tree.layer2.len() < 2 {
        return tree;
    }
    tree.top = Some(pk_top(&tree.layer2[0], &tree.layer2[1], coin));
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

    fn full_tree(colors: [GoddessColor; 8]) -> Vec<FragmentToken> {
        colors.map(|c| token(c, TrainingType::Speed)).to_vec()
    }

    #[test]
    fn parent_color_follows_the_left_fragment() {
        let slots = vec![
            token(GoddessColor::Red, TrainingType::Speed),
            token(GoddessColor::Blue, TrainingType::Stamina),
        ];
        let tree = build_with(&slots, &mut || true);
        assert_eq!(tree.layer1[0].color, GoddessColor::Red);
        assert_eq!(tree.layer1[0].stat_bonus, 3);
    }

    #[test]
    fn matching_colors_grant_bonus_2() {
        let slots = vec![
            token(GoddessColor::Red, TrainingType::Speed),
            token(GoddessColor::Red, TrainingType::Stamina),
        ];
        let tree = build_with(&slots, &mut || true);
        assert_eq!(tree.layer1[0].color, GoddessColor::Red);
        assert_eq!(tree.layer1[0].stat_bonus, 2);
    }

    #[test]
    fn training_inheritance_follows_the_coin() {
        let slots = vec![
            token(GoddessColor::Red, TrainingType::Speed),
            token(GoddessColor::Blue, TrainingType::Stamina),
        ];

        let left_wins = build_with(&slots, &mut || true);
        assert_eq!(left_wins.layer1[0].training, TrainingType::Speed);

        let right_wins = build_with(&slots, &mut || false);
        assert_eq!(right_wins.layer1[0].training, TrainingType::Stamina);
    }

    #[test]
    fn incomplete_pairs_do_not_merge() {
        let slots = full_tree([GoddessColor::Blue; 8])[..5].to_vec();
        let tree = build_with(&slots, &mut || true);
        assert_eq!(tree.bottom_slots.len(), 5);
        assert_eq!(tree.layer1.len(), 2);
        assert!(tree.layer2.is_empty());
        assert!(tree.top.is_none());
        assert!(!tree.is_complete());
    }

    #[test]
    fn top_pk_favors_more_same_color_backing() {
        let mut slots = full_tree([
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Blue,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
        ]);
        slots[1].training = TrainingType::Stamina;
        let tree = build_with(&slots, &mut || true);
        assert_eq!(tree.burst_color(), Some(GoddessColor::Yellow));
        assert!(tree.is_complete());
    }

    #[test]
    fn top_pk_tie_breaks_with_the_coin() {
        let slots = full_tree([
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Red,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
            GoddessColor::Yellow,
        ]);

        let left_wins = build_with(&slots, &mut || true);
        assert_eq!(left_wins.burst_color(), Some(GoddessColor::Red));

        let right_wins = build_with(&slots, &mut || false);
        assert_eq!(right_wins.burst_color(), Some(GoddessColor::Yellow));
    }
}
