# gm-tokens

Event-sourced tracker for the Grand Master scenario's token economy:
fragments, the pyramid tree (16 slots, the game's own layout), facility
points, goddess levels and the burst ledger. Pure logic, no game or UI
dependencies — any host (hachimi plugin, BepInEx plugin, simulator) records
events and reads a snapshot.

Mechanics and numbers follow the community "GM GUIDE" (Lehan); all raw data
lives in `src/tables.rs` so balance patches are a one-file edit.

## Model

State is never patched in place. A host records `RunEvent`s (a serialized
event list *is* a saved run), and `state::replay` rebuilds everything:

- `tree` — the game's 16-slot layout (`SingleModeScenarioVenusDefine`):
  slots 0–7 fragments, 8–11 layer 2, 12–13 layer 3, 14 the top merge, 15 the
  absorbed goddess, whose chara id (9040/9041/9042) *is* the burst color.
  `TreeSnapshot::from_packet` holds observed slots straight from a packet;
  `tree::predict` fills slots 8–15 from a bottom row using the guide's merge
  rules (left-color inheritance, 80/20 training inheritance, +3/+2 stat
  bonuses, top-node PK) through an injectable coin, so a seeded replay is
  identical every time.
- `facilities` — 2 points per click, 1 per matching fragment (+1 per 5),
  GM race points to all; one level per 12 points.
- `goddess_levels` — one level per burst of that color, capped at 5.
- `bursts` — the burst ledger, tagged with the phase they happened in.
- `advisories::evaluate(&snapshot)` — the guide's rules of thumb (golden
  slots 1 and 5, red-at-slot-1 overcap risk, yellow cap, 3-4-4-1 burst plan,
  URA force-red) as plain data; hosts decide how to display them.
- `mapper` — packet → events: `events_between` diffs two `VenusDataSet`s (the
  MessagePack `venus_data_set` the server sends each turn) with the game's
  own delta logic — new bottom slots are fragments, a grown active-effect
  array is a burst, a new race-history row is a GM race. Colour and attribute
  are not in the packet, so `MasterData` carries the two master-table joins
  (`spirit_id` → goddess, `effect_group_id` → attribute). Training clicks are
  the host's to emit; the packet does not carry them.

## Quick start

```rust
use gm_tokens::events::RunEvent;
use gm_tokens::{advisories, state};

let snapshot = state::replay_seeded(&events, 42);
for advisory in advisories::evaluate(&snapshot) {
    println!("{:?}: {}", advisory.kind, advisory.message);
}
```

A double fragment drop is two `FragmentGained` events on one turn; a failed
training records neither a click nor a fragment. When the game shows the
actual upper-layer rolls, `tree::predict_with` accepts an explicit coin.

## Tests

`cargo test` — tree merge rules with scripted coins, facility arithmetic,
burst overflow carry-over, goddess caps, advisory triggers, a scripted
junior year, and a round-trip pin of every guide number. Plain comments are
abolished: `tests/no_comments.rs` fails the build if one appears; short doc
comments on public items are the allowed exception.
