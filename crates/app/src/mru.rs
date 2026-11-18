//! The Most Recently Used commands of the Plot, Worksheet, Analysis and Statistics menus (T78,
//! `docs/spec/recent-lists.md`).
//!
//! A command that ran from one of those menus is kept as a text key: the menu name and the labels
//! of the submenus and of the item, joined with ` > `. A key is not a `Cmd` (which cannot be saved);
//! when an entry is drawn or chosen, the key is looked up again in the table of the current bar and
//! the command is resolved against the current selection, exactly as a click on the item itself.
//! A key the table no longer has is not shown.

use crate::menus::{Cmd, Entry, Item, Menu, plain};

/// A menu that has a Most Recently Used list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MruMenu {
    Plot,
    Worksheet,
    Analysis,
    Statistics,
}

impl MruMenu {
    /// The menu of that name (as the bar shows it, without `&`).
    pub fn from_name(name: &str) -> Option<MruMenu> {
        [MruMenu::Plot, MruMenu::Worksheet, MruMenu::Analysis, MruMenu::Statistics]
            .into_iter()
            .find(|m| m.name() == name)
    }

    /// The name of the top-level menu; also the name of the list in `recent.json` (lower case).
    pub fn name(self) -> &'static str {
        match self {
            MruMenu::Plot => "Plot",
            MruMenu::Worksheet => "Worksheet",
            MruMenu::Analysis => "Analysis",
            MruMenu::Statistics => "Statistics",
        }
    }

    /// The key of the list in `recent.json`.
    pub fn list_name(self) -> String {
        self.name().to_lowercase()
    }
}

/// Whether a command is worth remembering: the menu housekeeping items are not.
pub fn recordable(cmd: &Cmd) -> bool {
    !matches!(cmd, Cmd::ManageTemplates | Cmd::ClearRecent(_) | Cmd::ClearRecentExports)
}

/// Calls `f` with the key and the item of every item of `menu`'s entries, until it returns true.
fn each<'a>(entries: &'a [Entry], path: &mut Vec<String>, f: &mut dyn FnMut(&str, &'a Item) -> bool) -> bool {
    for e in entries {
        match e {
            Entry::Item(i) => {
                path.push(plain(&i.label));
                let stop = f(&path.join(" > "), i);
                path.pop();
                if stop {
                    return true;
                }
            }
            Entry::Submenu(s) => {
                path.push(plain(&s.label));
                let stop = each(&s.entries, path, f);
                path.pop();
                if stop {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn top(menus: &[Menu], menu: MruMenu) -> Option<&Menu> {
    menus.iter().find(|m| m.name() == menu.name())
}

/// The key of the first item of `menu` that runs `cmd`.
pub fn key_of(menus: &[Menu], menu: MruMenu, cmd: &Cmd) -> Option<String> {
    let m = top(menus, menu)?;
    let mut found = None;
    each(&m.entries, &mut vec![menu.name().to_string()], &mut |key, item| {
        let hit = item.cmd.as_ref() == Some(cmd);
        if hit {
            found = Some(key.to_string());
        }
        hit
    });
    found
}

/// The item a key names, if the bar still has it.
pub fn find<'a>(menus: &'a [Menu], menu: MruMenu, key: &str) -> Option<&'a Item> {
    let m = top(menus, menu)?;
    let mut found = None;
    each(&m.entries, &mut vec![menu.name().to_string()], &mut |k, item| {
        let hit = k == key && item.cmd.is_some();
        if hit {
            found = Some(item);
        }
        hit
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::{WindowKind, bar};
    use sheaf_core::template::Template;

    #[test]
    fn a_key_finds_the_item_it_came_from() {
        let menus = bar(WindowKind::Workbook);
        let cmd = Cmd::Plot(Template::Line);
        let key = key_of(menus, MruMenu::Plot, &cmd).expect("the Plot menu has Line");
        assert!(key.starts_with("Plot > "), "{key}");
        let item = find(menus, MruMenu::Plot, &key).expect("found again");
        assert_eq!(item.cmd.as_ref(), Some(&cmd));
    }

    #[test]
    fn keys_tell_apart_items_that_share_a_label() {
        let menus = bar(WindowKind::Workbook);
        let a = key_of(menus, MruMenu::Worksheet, &Cmd::SortRange { descending: false }).unwrap();
        let b = key_of(menus, MruMenu::Worksheet, &Cmd::SortColumns { descending: false }).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn an_unknown_key_and_a_command_of_another_menu_give_nothing() {
        let menus = bar(WindowKind::Workbook);
        assert!(find(menus, MruMenu::Plot, "Plot > No Such Plot").is_none());
        assert!(key_of(menus, MruMenu::Statistics, &Cmd::Plot(Template::Line)).is_none());
        assert!(!recordable(&Cmd::ManageTemplates));
        assert!(recordable(&Cmd::Plot(Template::Line)));
    }

    #[test]
    fn the_menu_names_round_trip() {
        for m in [MruMenu::Plot, MruMenu::Worksheet, MruMenu::Analysis, MruMenu::Statistics] {
            assert_eq!(MruMenu::from_name(m.name()), Some(m));
        }
        assert_eq!(MruMenu::from_name("File"), None);
        assert_eq!(MruMenu::Analysis.list_name(), "analysis");
    }
}
