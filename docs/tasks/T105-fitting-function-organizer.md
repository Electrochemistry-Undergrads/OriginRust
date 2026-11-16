# T105: Fitting Function Organizer

**Status:** done 2026-11-16 (Claude Windows "t30-origin", fitting.md "Fitting Function Organizer"): Origin's organizer measured; Sheaf's browses the built-in library by category with each function's definition, read-only. Left: user functions (Builder, New/Save), typeset equation, sample curve.
**Size:** S. **Source:** greyed inventory package B13 (Tools > Fitting Function Organizer... (F9)).
**Spec:** `docs/spec/fitting.md` (a new section).

## Goal

Tools > Fitting Function Organizer (F9) shows the fit functions Sheaf has, by category, with each
function's formula, parameters and derived parameters, as Origin 2016's organizer browses its library.
Adding and editing user functions (Origin's Fitting Function Builder and the organizer's New / Save) is
not part of this slice.

## Origin facts

Measure in Origin 2016: the organizer window (tree of categories and functions, the fields shown for a
function, the buttons).

## Done when

The item is enabled and shows the built-in library as measured, fmt/clippy/tests pass, inventory and
spec updated.
