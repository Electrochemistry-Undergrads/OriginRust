# T283: Window comments in .opj files

**Status:** done 2026-11-21 (Claude Windows "claude:3"). HANDOFF next candidates B (T249's left item).
Measured in Origin 2016 (`re-workspace/scratch/t282`, local): `page.comments$` on books, a matrix and
graphs saves the comments in the window label after the long name and a CR LF (`BKA\r\nZQXbookA`,
`\r\nZQXmatrix`, `GLong\r\nZQXg2`). The `.opj` reader now reads them into `Project::window_comments`
and the writer writes them for workbooks and graphs, so the save no longer warns or needs the
companion for them; notes, layout and matrix windows, which the writer leaves out, still use the
companion.
**Size:** S. **Spec:** `docs/spec/opj-format.md` section 4 (label at `0xC3`), `docs/spec/ui-command-map.md`
(Window Properties).

## Tests

- `crates/io/src/opj/window.rs`: `the_label_holds_the_long_name_then_the_comments`.
- `crates/io/src/opj/write/graph_tests.rs`: `window_comments_go_in_the_label_after_the_long_name`.
- `crates/io/src/opj/tests.rs`: `local_origin_file_gives_the_window_comments` (Origin's file, local).
- `crates/io/src/save/tests.rs`: `window_comments_of_books_go_in_the_opj_itself`.
