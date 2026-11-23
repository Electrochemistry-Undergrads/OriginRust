# T398: EC50 marks on dose-response fit graphs

**Status:** done 2026-11-23 (Claude Windows "claude:gmail", fitting "EC50 marks"). HANDOFF C (Origin 2027 Pro "mark
IC50/EC50 on dose-response fits"): an opt-in dashed crosshair at the EC50 (on the plot's X scale) and the half response
on the graph of a DoseResp, Logistic or Logistic5 fit: a Fit-window checkbox, `sheaf fit --plot ... --mark-ec50` and
`Project::mark_ec50`. Origin 2016's fit graphs stay as measured (no marks by default). Not measured in Origin.
**Size:** S. **Spec:** fitting "EC50 marks".

## Left

- Marks for global and multiple fits, EC20/EC80 levels, a label that shows the EC50 itself rather than its log.

## Tests

- `crates/core/src/fit_marks.rs`: the marks of a DoseResp fit at LOGx0 and the half response; a straight line refused.
- `crates/cli/tests/fit.rs`: `a_dose_response_fit_marks_its_ec50_on_the_figure`.
- `crates/app/src/app/ui_tests.rs`: `the_fit_window_marks_the_ec50_of_a_dose_response_fit`.
