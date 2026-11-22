# T357: Integrate gadget Preferences

**Status:** done 2026-11-22 (Claude Windows "claude:gmail", graphing "Integrate gadget Preferences"). The parity row
"Integrate range, baseline, charge" (P0) said the Integrate gadget had no settings dialog. Its menu now has
Preferences...: X From and X To typed, the baseline (zero or the end-point line) and the area type (Mathematical or
Absolute); OK applies them to the gadget, whose live value and New Output follow. Origin's own dialog was not
measured (GUI only).
**Size:** S. **Spec:** graphing, Integrate gadget Preferences.

## Left

- Origin's output options (which quantities, where), the gadget's label settings, baselines other than zero and the
  end-point line.

## Tests

- `crates/app/src/app/ui_tests.rs`: `the_integrate_gadget_preferences_set_the_range_baseline_and_area_type` (menu
  entry, window, range applied, absolute area 25 against a signed 0 on y = x - 5).
