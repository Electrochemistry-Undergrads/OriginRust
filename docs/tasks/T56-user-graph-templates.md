# T56: Save Template As and user graph templates

**Status:** claimed 2026-11-15 (branch `user-templates`, Claude macOS "mac-templates"). Brief written from parity
row "Save Window As, Save Template As, analysis template" (Save Template As still grey after T54), T18's
"Templates (save/load)" row and the owner's wish that figures keep their own style (HANDOFF "Needs from the
owner" 2).
**Size:** M. **Branch:** `user-templates`. **Model:** Sonnet. No Origin needed for the mechanism (Sheaf's own
file kind); Origin's template rules are measured once on Windows.
**Spec:** a new section in `docs/spec/graphing.md` ("User templates"); `docs/spec/window-files.md` for the
file layout it shares with T54.

## Goal

On a graph, File > Save Template As... stores the graph's look without its data: page size and colours,
layers and their placement, axes (scales, ranges or automatic, ticks, labels, fonts, breaks), each plot's
style in order (kind, line, symbol, colours, fill, error-bar style), legend and text objects (with the
`%(...)` / column-label substitutions kept as text), drawn objects. The template gets a name and lands in
the user template folder; Plot > User Templates lists the saved ones (beside the built-in "Cyclic
voltammetry overlay"), and choosing one plots the selected worksheet columns in that look: the first
selected Y gets the first plot style, and so on (styles repeat or follow the template's group rules when
there are more columns). A Manage... item (rename, delete, reveal folder) keeps the list tidy. `sheaf plot
--template NAME` uses the same templates.

## Origin facts

- Not measured. From Origin's public documentation (paraphrased): File > Save Template As saves the active
  graph as a template (`.otp`/`.otpu`), without data; Plot > Template Library and the user templates list
  plot new data with it; the number of plots in the template and how extra datasets are styled follow the
  template's plot groups. Origin's template files are Origin's own format: Sheaf neither reads nor writes
  them. Measure on Windows: what a template keeps (axis ranges fixed or rescaled? text objects? legend?),
  how extra or fewer datasets are styled, and the default template folder/name. Until then Sheaf's choices,
  labelled: axis ranges stay as saved if they were fixed and rescale if they were automatic; text objects
  and drawn objects are kept; extra datasets cycle the saved styles with the template's colour order.
- Analysis templates (workbooks) and graph themes (.OTH, Theme Organizer) are separate (T18); keep them grey.

## Where

- `crates/core`: a `UserTemplate` built from a `Graph` (the graph with its plot data references removed or
  replaced by slots), and `Project::plot_with_user_template(template, columns)`; reuse the code built-in
  templates use to create graphs (find `Template` and how `Plot(Template::…)` makes a graph) so a user
  template is a data-less graph applied the same way.
- `crates/io`: the template file (the `.sheaf` zip with a template marker, like T54's window marker; name
  `<name>.sheaftemplate`) in the settings folder `templates/` (like T46's `import-themes`), list, rename,
  delete, tolerant load; never stores worksheet data.
- `crates/app`: enable Save Template As... on graphs (name prompt; default the graph's short name), the
  Plot > User Templates submenu filled from the folder, Manage...; `Startup.templates_dir` like T46 (tests
  pass a temp folder or `None`).
- `crates/ops` / CLI: `sheaf plot --template NAME` (and `--templates-dir`); MCP's plot tool gains the same
  through the shared clap options if they flow automatically, else document as left.

## Steps

1. Spec section (Sheaf's choices labelled; Origin measurements listed).
2. Core template build/apply + io file with tests (round trip keeps every look setting; data never stored;
   two, three and five datasets on a two-plot template; double-Y template; fixed vs automatic ranges;
   text object substitutions re-evaluated against the new data).
3. App menus, prompt, submenu, Manage; UI tests: save a styled graph as a template, plot other columns with
   it, the new graph's look equals the saved one; rename/delete; templates survive a restart; real folder
   never touched in tests.
4. CLI flag with tests; parity row (Save Template As to ◐), T18 note.

## Done when

A styled graph saved as a template plots other data with the same look from the menu and the CLI, the
template list survives a restart, fmt, clippy and tests pass.

## Don't

- Don't read or write Origin's `.otp` files.
- Don't store worksheet data in a template.
- Don't change the built-in templates' behaviour.
