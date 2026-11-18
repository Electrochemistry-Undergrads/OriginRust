# T132: Tools > Fitting Function Builder

**Status:** done 2026-11-18 (Claude Windows "t30-origin", fitting.md "Fitting Function Builder"), **not
measured** (Origin COM unregistered on the test machine). Left: Origin's multi-page wizard (variables,
bounds, derived parameters, Origin C bodies, categories in the organizer); measure and match.
**Size:** S. **Source:** greyed inventory package B13 (Tools > Fitting Function Builder..., F8).
**Spec:** `docs/spec/fitting.md` ("Fitting Function Builder").

## Goal

Define a fit function of one's own (name, parameters with initial values, y = f(x)) and save it as an
Origin `.FDF` file that the Nonlinear Curve Fit's .FDF category reads.

## Done when

F8 opens the builder, Save writes a file that reads back as the same function, the inventory no longer
lists the item, and fmt, strict Clippy and the full tests pass.
