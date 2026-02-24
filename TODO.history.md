# Text-based System Changelog

Users should be able to view a rough overview of the latest system changes in a simple text-based diff format.

The format should be easily readable for the general public, because they are not programmers and are not used to the green/red diff view.

The diff should be based on changes in the CustomFronts + Members dump of the system.

## Trigger Mechanism

Whenever the existing `relevantly_changed_based_on_simply_plural_websocket_event()` function detects a relevant change, the system is marked for a **full fetch** of all CustomFronts and Members.

Rate limiting (use existing `RateLimitedSend` implementation):
- `wait_increment: 5s`
- `wait_max: 30m`
- `duration_to_count_over: 2h`

Changes should be **batched** within the rate-limiting window.

Rate limiting uses **hardcoded defaults** (not user-configurable).

## Storage Model

- **Latest snapshot:** Stored to compute new deltas. Replaced on each update.
- **History:** Only **deltas** are stored for a long time. For full snapshots, only the latest snapshot is stored of the entire system.
- **Per-system changelog** (not global).
- **No backfill** when feature is first enabled—only track changes going forward.
- Furthermore, only the last X entries and N days will be stored. The logic for storage and pruning and the configuration will be equivalent to how it's already implemented for history fronting.

## Data Handling

## Delta-computation

Given a stored system snapshot A and a new system snapshot B (freshly fetched),
where the snapshot consists of a Map<MemberId, JSON> and a Map<CustomFrontId, JSON> (both extracted from the HTTP requests), then the diff is computed as follows:
* for all MemberId/CustomFrontId where the id only appears in either A or B,
  then mark the corresponding IDs are added/removed.
* for all MemberId/CustomFrontId where both snaphots have JSONs,
  then compare both JSONs (and truncate long fields) and save only the changed fields. e.g if A: {name: hello, age: 20} and B: {name: holla, age: 20}
  then the delta is stored as {old: {name: hello}, new: {name: holla}}.
* save the new snapshot B into the database over the old snapshot A. (for each PluralSync user, there is only a single full snapshot stored.)

### Truncation & Hashing
- All potentially unbounded string fields must be **truncated and hashed** if they exceed a certain limit (e.g., 10,000 characters).
- Display format: `truncated + hash` (e.g., `[truncated: abc123...]`).
- This limits database storage size.

### Field Lookups (for interpretable display)
- `system_id` → Lookup and store **name + UID**, display both.
- `privacy_buckets` → Resolve bucket IDs to **bucket names**.
- `pluralkit_id` → Lookup **PluralKit member names**.

Lookups should happen **at fetch time** so the changelog is self-contained and doesn't degrade over time if members are deleted.

## Database

- **New separate table** (e.g., `system_changelog`), not merged with existing `history_status`.

## Frontend Display

- The history tab already exists and shows fronting history.
- Add the **system changelog** in the same location.
- For now: **Fetch the entire history and ignore pagination** for simplicity.
- Displayed **nearby** fronting history, but they are **strictly separate features**.
- Frontend UI layout details to be determined later.

