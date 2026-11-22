# T369: Acceptance sampling by attributes

**Status:** done 2026-11-22 (Claude Windows "claude:1", statistics 39). HANDOFF C (Origin 2026b Pro app
"Acceptance Sampling"): Statistics > Acceptance Sampling... and `sheaf analyze acceptance` design the single
sampling plan with the smallest sample meeting the producer's and consumer's risks at the AQL and LTPD, or
evaluate a plan given by hand, with the binomial OC curve and, with a lot size, AOQ, AOQL and ATI; a new
workbook and an OC graph. Checked against scipy, not measured in Origin.
**Size:** S. **Spec:** statistics 39.

## Tests

- `crates/calc/tests/acceptance_reference.rs`: five designs (brute force), OC/AOQ/ATI at seven fractions for four
  plans, four AOQLs, against `tools/acceptance-ref.py`.
- `crates/calc/src/acceptance.rs`: refusals, one item less fails the consumer's risk, AOQ is largest at the AOQL.
- `crates/core/src/analysis/acceptance.rs`: the book, sheets and graph; a plan by hand; a too small lot.
- `crates/cli/tests/analyze.rs`: `acceptance_sampling_designs_and_evaluates_plans_without_a_data_file`.
- `crates/app/src/app/ui_tests.rs`: `acceptance_sampling_designs_a_plan_into_a_new_book_with_its_oc_graph`.

## Left

- Double, multiple, sequential and variables plans; hypergeometric (type A) curves; ISO 2859 / MIL-STD-105E
  tables; an MCP tool; Origin's own layout.
