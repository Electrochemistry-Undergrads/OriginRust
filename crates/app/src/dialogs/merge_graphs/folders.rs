//! Captured Explorer scope for the two measured folder merge modes (also Export Graphs' folder
//! choices, T250).

use crate::panels::explorer;
use sheaf_core::project::Folder;
use sheaf_core::{Project, WindowId};
use std::collections::{BTreeMap, HashSet};

pub(crate) struct FolderScope {
    path: Vec<usize>,
    // Folders have no stable ids. Refuse a changed tree rather than retargeting an index path.
    root: Folder,
}

impl FolderScope {
    pub(crate) fn new(project: &Project, path: &[usize]) -> Self {
        Self { path: path.to_vec(), root: project.root.clone() }
    }

    pub(crate) fn valid(&self, project: &Project) -> bool {
        self.root == project.root && explorer::folder_at(&project.root, &self.path).is_some()
    }

    pub(crate) fn graphs(&self, project: &Project, recursive: bool) -> Vec<WindowId> {
        if !self.valid(project) {
            return Vec::new();
        }
        let mut by_folder: BTreeMap<Vec<usize>, Vec<WindowId>> = BTreeMap::new();
        for graph in project.graphs() {
            let home = explorer::home(&project.root, graph.id);
            if home == self.path || (recursive && home.starts_with(&self.path)) {
                by_folder.entry(home).or_default().push(graph.id);
            }
        }
        fn visit(
            project: &Project,
            path: &mut Vec<usize>,
            recursive: bool,
            by_folder: &BTreeMap<Vec<usize>, Vec<WindowId>>,
            out: &mut Vec<WindowId>,
        ) {
            if let Some(graphs) = by_folder.get(path) {
                out.extend(graphs);
            }
            if recursive {
                let folder = explorer::folder_at(&project.root, path).expect("validated folder path");
                let mut children: Vec<_> = folder.folders.iter().enumerate().collect();
                children.sort_by_cached_key(|(_, f)| f.name.to_lowercase());
                for (index, _) in children {
                    path.push(index);
                    visit(project, path, true, by_folder, out);
                    path.pop();
                }
            }
        }
        let mut out = Vec::new();
        visit(project, &mut self.path.clone(), recursive, &by_folder, &mut out);
        out
    }

    pub(super) fn graphs_with_embedded(&self, project: &Project) -> Vec<WindowId> {
        if !self.valid(project) {
            return Vec::new();
        }
        let embedded = project
            .workbooks()
            .filter(|book| explorer::home(&project.root, book.id) == self.path)
            .flat_map(|book| &book.sheets)
            .flat_map(|sheet| sheet.graph_sheet.into_iter().chain(sheet.cell_graphs.iter().map(|cell| cell.graph)))
            .filter(|&id| project.graph(id).is_some());
        let mut seen = HashSet::new();
        embedded.chain(self.graphs(project, false)).filter(|id| seen.insert(*id)).collect()
    }
}
