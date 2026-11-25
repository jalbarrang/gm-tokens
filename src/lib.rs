//! Event-sourced Grand Master token tracker: fragments, the pyramid tree,
//! facilities, goddess levels and bursts. Hosts record [`events::RunEvent`]s;
//! the crate rebuilds all state from them, so an event list is a saved run.

pub mod advisories;
pub mod domain;
pub mod events;
pub mod state;
pub mod tables;
pub mod tree;
