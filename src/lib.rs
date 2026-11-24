//! # gm-tokens
//!
//! Event-sourced tracker for the Grand Master scenario's token economy:
//! fragments, the 8-slot pyramid (binary) tree, facility points, goddess
//! levels and the burst ledger.
//!
//! The crate is pure logic with no game or UI dependencies, so any host
//! (hachimi plugin, BepInEx plugin, simulator) can feed it events and read
//! a snapshot. State is never patched in place; it is always rebuilt from
//! the event list, so persisting a run means serializing its events.
//!
//! Mechanics and numbers follow the community "GM GUIDE" (Lehan), see
//! [`tables`] for the raw data and [`advisories`] for its rules of thumb.
//!
//! ```
//! use gm_tokens::events::RunEvent;
//! use gm_tokens::{advisories, state};
//!
//! let events = vec![
//!     RunEvent::PhaseStarted { turn: 1, phase: gm_tokens::domain::CareerPhase::Junior },
//!     RunEvent::TrainingClick { turn: 1, training: gm_tokens::domain::TrainingType::Speed },
//!     RunEvent::FragmentGained {
//!         turn: 1,
//!         color: gm_tokens::domain::GoddessColor::Blue,
//!         training: gm_tokens::domain::TrainingType::Speed,
//!         source: gm_tokens::domain::FragmentSource::Training,
//!     },
//! ];
//! let snapshot = state::replay_seeded(&events, 42);
//! assert_eq!(snapshot.tree.bottom_slots.len(), 1);
//! assert!(!advisories::evaluate(&snapshot).is_empty());
//! ```

pub mod advisories;
pub mod domain;
pub mod events;
pub mod state;
pub mod tables;
pub mod tree;
