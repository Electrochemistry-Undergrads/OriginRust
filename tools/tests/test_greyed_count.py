"""Regressions for inventory counting and stale-document detection."""
import importlib.util
from pathlib import Path
import re
import unittest

SPEC = importlib.util.spec_from_file_location("greyed_count", Path(__file__).resolve().parents[1] / "greyed-count.py")
COUNT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COUNT)
SAMPLE = """# Inventory
## Menu bar
### File
- `T58` Titles: X · Y [Workbook, Graph]
## Toolbars
- `T59` Font: Bold
## Context menus
- `B1` View: Zoom In · Zoom Out
"""


def sample_files():
    inventory = SAMPLE.replace("# Inventory\n", "# Inventory\n**5 greyed items: 2 in the menu bar, 1 on the toolbars, 2 in context menus.**\n" + COUNT.START + "\n" + COUNT.END + "\nBriefs (`T`) hold 3 items, backlog packages (`B`) 2, not planned (`N`) 0.\n")
    plan = """# Plan for the 5 greyed items
| ✓ | Package | What | Size | Greyed items |
|---|---|---|---|---|
| [ ] | [T58](T58-text.md) | Text | L | 2 |
| [ ] | [T59](T59-cells.md) | Cells | L | 1 |
| [ ] | B1 | View | M | 2 |
## Not planned (`N`, 0 items)
"""
    handoff = "**5 listed item locations** at the planning\n  baseline (2 menu bar, 1 toolbar, 2 context menu)\n"
    return {"docs/tasks/greyed-items.md": inventory, "docs/tasks/PLAN.md": plan, "HANDOFF.md": handoff}


class InventoryTests(unittest.TestCase):
    def test_counts_locations_without_multiplying_window_qualifiers(self):
        packages, surfaces = COUNT.count_items(SAMPLE)
        self.assertEqual(sum(packages.values()), 5)
        self.assertEqual(packages["T58"], 2)
        self.assertEqual(surfaces["Menu bar"], 2)

    def test_unknown_package_cannot_silently_disappear(self):
        with self.assertRaisesRegex(ValueError, "unknown package"):
            COUNT.count_items(SAMPLE.replace("`T58`", "`T580`"))

    def test_missing_tag_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "missing or unknown"):
            COUNT.count_items(SAMPLE.replace("`T58` ", ""))

    def test_empty_label_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "empty item"):
            COUNT.count_items(SAMPLE.replace("X · Y", "X ·  · Y"))

    def test_missing_surface_is_an_error(self):
        with self.assertRaisesRegex(ValueError, "all three"):
            COUNT.count_items(SAMPLE.split("## Context menus")[0])

    def test_surface_can_be_empty_after_its_last_item_is_enabled(self):
        packages, sections = COUNT.count_items(SAMPLE.replace("- `T59` Font: Bold\n", ""))
        self.assertEqual(packages["T59"], 0)
        self.assertEqual(sections["Toolbars"], 0)

    def test_backlog_can_be_reassigned_to_a_new_brief(self):
        paths = ("docs/tasks/greyed-items.md", "docs/tasks/PLAN.md", "HANDOFF.md")
        source = sample_files()
        source[paths[0]] = source[paths[0]].replace("- `B1` ", "- `T64` ")
        source[paths[1]] = source[paths[1]].replace("| B1 |", "| [T64](T64-graph-view.md) |")
        updated, packages, _ = COUNT.refreshed(source)
        self.assertEqual(packages["T64"], 2)
        self.assertNotIn("B1", packages)
        self.assertEqual(updated, COUNT.refreshed(updated)[0])

    def test_refresh_detects_stale_counts_and_is_idempotent(self):
        paths = ("docs/tasks/greyed-items.md", "docs/tasks/PLAN.md", "HANDOFF.md")
        source = sample_files()
        source[paths[0]] = re.sub(r"\d+ greyed items", "1 greyed items", source[paths[0]], count=1)
        updated, _, _ = COUNT.refreshed(source)
        self.assertNotEqual(source[paths[0]], updated[paths[0]])
        self.assertEqual(updated, COUNT.refreshed(updated)[0])
        with self.assertRaisesRegex(ValueError, "unknown/duplicate plan package"):
            duplicate = dict(updated)
            row = next(line for line in updated[paths[1]].splitlines() if line.startswith("| [ ] | [T58]"))
            duplicate[paths[1]] += row + "\n"
            COUNT.refreshed(duplicate)


if __name__ == "__main__":
    unittest.main()
