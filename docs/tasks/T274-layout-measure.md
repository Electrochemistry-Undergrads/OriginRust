# T274: Layout page measured in Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:2"), partly measured.
HANDOFF "Next candidates" B: measure T237's unmeasured layout pages. In Origin 2016, `win -t layout`
makes a page with `page.width` 6400, `page.height` 4800, `page.resx`/`resy` 600 (10.667 x 8 in, 768 x
576 pt; the graph page of the same run is 6432 x 4923). Sheaf's default was US Letter landscape (792 x
612); it is now 768 x 576. Where Add Graph puts a graph could not be measured: `layout Graph1`,
`layout -g/-t/-a Graph1` all fail and `page -g` adds nothing (exported pictures blank), so it needs the
dialog (screenshots), left for later.
**Size:** S. **Spec:** `docs/spec/layout-page.md` (Model). Probe scripts and outputs:
`re-workspace/scratch/layout-measure/` (local only).

## Tests

- `layout_page::tests` (core) checks the default; the render and Page Setup tests set or pick Letter.
