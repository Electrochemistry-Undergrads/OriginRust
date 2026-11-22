# T331: Auto Parameter Initialization on and off in nonlinear fits

**Status:** done 2026-11-21 (Claude Windows "claude:2"). Origin's values with the tick off not measured.
Feature-parity "Automatic initial guess" (P0) gap "no toggle". Origin's NLFit has the tick "Auto Parameter
Initialization" (on). Sheaf's `FitRequest::auto_init` (default on): off, curve and surface fits skip the
guess and start from the typed values, else the function's own starting values. Window: the tick above the
Parameters table; CLI `sheaf fit --no-auto-init`.
**Size:** S. **Spec:** `docs/spec/fitting.md` 5 (Auto Parameter Initialization).

## Tests

- `fitting::tests::auto_parameter_initialization_can_be_turned_off` (core),
  `the_request_carries_only_the_settings_that_were_made` (app), `auto_parameter_initialization_can_be_turned_off` (cli).

## Left

- Measure what Origin's table holds with the tick off (drive the NLFit Parameters grid); the tick in MCP.
