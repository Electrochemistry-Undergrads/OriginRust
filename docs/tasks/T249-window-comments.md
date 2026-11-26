# T249: Window Properties Comments

**Status:** done 2026-11-20 (Claude Windows "claude:3"), not measured. Window Properties (Alt+Enter,
Window > Properties, the Explorer's Properties...) gets Comments for every window kind, kept in
`Project::window_comments` (one undo step with the names), shown as the Explorer row's tooltip, gone
with the window; an `.opj` save names them and writes the companion.
**Size:** S. **Source:** feature-parity "Window properties and rename" (P1) gap.
**Spec:** `docs/spec/ui-command-map.md` (Window Properties, after T220).

## Tests

- `crates/core/src/project.rs`: `window_comments_are_kept_per_window_and_go_with_the_window`.
- `crates/app/src/app/ui_tests.rs`: `window_properties_keep_comments_with_undo`.
- `crates/io/src/save/tests.rs`: `window_comments_need_the_companion`.

## Left

- Origin's Window Properties layout and Window Title options; a Comments column in the Explorer list (done in T350).
  Comments in `.opj` were measured and done in T283.
