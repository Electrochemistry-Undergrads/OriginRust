# T95: Find in Project

**Status:** done 2026-11-16 (Claude Windows "t30-origin", workbook-model.md 13j): Origin's Find window, defaults and hits measured; Sheaf searches graph, book, sheet and column names and labels, data cells and graph text with the same switches, and goes to a hit. Left: Notes, folder info variables, a separate Folder result column and Origin's double-click.

Worksheet Comments added 2026-11-18. The archived Origin 2016 Select Metadata window shows Comments
checked under Sheet. Sheaf searches the full multiline comment and opens its sheet from the result.
The result follows Sheaf's existing sheet hit behavior. Formatting, strict workspace Clippy, Nextest (4,520 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.
Desktop captures verified the default Comments checkbox, toggling it off and keeping that setting
when Find is reopened. The automated UI test verified search results and navigation.

Within added 2026-11-18. The default Entire Project matches the archived capture. Active Folder
and Active Folder and Subfolders follow the Origin documentation and use the selected Project
Explorer folder. Formatting, strict workspace Clippy, Nextest (4,528 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.
Desktop captures verified the active path, Within choices and selected subtree scope.
Automated tests verified result counts, navigation to a hidden descendant and the session setting.

Folder Name and Comments added 2026-11-18 using the archived Select Metadata defaults. Folder
matches respect Within and clicking a hit activates its folder. Formatting, strict workspace Clippy, Nextest (4,531 passed and
8 skipped), doc tests, inventory and eight Python tooling tests passed.
Desktop captures verified Folder Comments was checked by default and retained its changed setting
when Find was reopened. The automated UI test verified matches and folder activation.

**Size:** M. **Source:** greyed inventory package B7 (Edit > Find in Project... on a workbook, Find in
Project on a graph; F3).
**Spec:** `docs/spec/gui-look.md` section 5 (Find) or `workbook-model.md`.

## Goal

Find in Project (F3) searches the whole project for a text, as Origin 2016's: the window, what it searches
(window names and long names, column names and labels, cells, graph text) and how a hit is shown and
opened.

## Origin facts

Measure in Origin 2016: the window's fields and defaults, what kinds of objects it searches, the result
list's columns, and what activating a result does.

## Done when

Both items and F3 are enabled and behave as measured (fixtures), fmt/clippy/tests pass, inventory and
spec updated.
