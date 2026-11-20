# T211: Cell links: Paste Link, Paste Link Transpose, Remove Links (B5 slice)

**Status:** done 2026-11-20 (Claude Windows "claude:1"), **not measured**. Worksheet cells can be linked to
the cells last copied (Edit and data cell Paste Link / Paste Link Transpose); linked cells follow their
sources inside every edit's undo step; Remove Links (Edit, data cell, column header) keeps the values.
**Size:** M. **Source:** greyed inventory package B5 (Paste Link, Paste Link Transpose, Remove Links).
**Spec:** `docs/spec/workbook-model.md` 13j0; [ADR 0014](../adr/0014-cell-links.md).

## Left

- Measure Origin's links (storage as cell formulas, paste rules, display); `.opj` round trip of links.
- The graph window's Paste Link; Paste Special.
