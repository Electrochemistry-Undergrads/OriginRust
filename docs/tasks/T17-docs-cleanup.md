# T17: Stale spec paragraphs and parity counts

**Status:** open. **Size:** S. **Branch:** `docs-fix` (or commit on `main` directly if nothing else
merges at the time). **Model:** Haiku or Sonnet. Docs only; no code.

1. `docs/spec/feature-parity.md` section 7 says peak results lack area percent and FWHM; both exist
   (`crates/core/src/analysis.rs`, search "FWHM"). Another line of the same section already says FWHM
   per peak. Fix the row.
2. `docs/spec/gui-look.md` section 7 predates the grid work: the grid now has Long Name, Units,
   Comments and F(x)= rows, group numbers in headers ("A(X1)") and 5-decimal display. Sparklines and
   Parameters label rows are still missing. Rewrite the "Sheaf today" part.
3. `crates/core/src/graph.rs` `AutoRange` doc says "with a small margin"; the code has none. Wait for
   `axis-range` to merge (it adds the margin), then make the doc state the rule it implements.
4. `docs/spec/import-ascii.md` section 15: record the Origin 2016 measurement that mixed text/number
   columns stay numeric with format 7 (Text & Numeric), text cells kept (95 % and 55 % numeric
   columns both; `cell(11,2)$ = four`; files `re-workspace/scratch/audit-2/mix2.*`). Coordinate with
   branch `text-numeric`, which may already do it.
5. Recount the parity table: `python tools/parity-count.py`, and update the counts in HANDOFF.md
   (2026-11-12 night: 41 done, 109 partial, 80 missing of 230; P0 31 / 70 / 0).
6. `docs/ROADMAP.md` Milestone 6 "To do" still lists area plots and the left/right axis control, both
   done; tick or reword.

Done when: each fixed; `git diff` reviewed; commit `docs: ...`.
