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
- **History:** Only **deltas** are stored (not full snapshots).
- **Per-system changelog** (not global).
- **No backfill** when feature is first enabled—only track changes going forward.

## Data Handling

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

## Security

- **TODO:** Encrypt the changelog like platform secrets/tokens (using `pgp_sym_encrypt`/`pgp_sym_decrypt` as in `user_config_queries.rs`). Currently stored as plaintext.

## Frontend Display

- The history tab already exists and shows fronting history.
- Add the **system changelog** in the same location.
- For now: **Fetch the entire history and ignore pagination** for simplicity.
- Displayed **nearby** fronting history, but they are **strictly separate features**.
- Frontend UI layout details to be determined later.

