# T301: Measure File > Import > Image against Origin 2016

**Status:** done 2026-11-21 (Claude Windows "claude:fe4b", matrices 7h). Origin's `impImage` always makes
a new matrix book. Its short name is the file's stem (numbered when taken), its long name the file name
with the extension, and a gray picture's sheet is named after the stem. X runs from 0 to cols - 1 and Y
from 0 to rows - 1, the top picture row first. Gray levels are kept; a colour picture as data is
round((2R + 5G + B) / 8). Sheaf now names and maps the book the same way and keeps its colour channels as
sheets (T218).
**Size:** S. **Follows:** T204, T218. **Spec:** matrices 7h.

## Left

- Origin keeps a colour picture as one RGB image; Sheaf keeps Red, Green and Blue sheets (Color to Gray
  gives Origin's gray values).
- TIFF and the other formats Origin reads.

## Tests

- `crates/app/src/app/ui_tests/matrix_window.rs`: `an_imported_picture_becomes_a_matrix_of_gray_levels`
  (names, numbering, ranges).
- `crates/core/src/project.rs`: `Project::unique_name_from`.
