//! UpSet plot counts (T403, Origin 2027's UpSet plot; Sheaf's own rules, not measured): rows of membership flags,
//! one flag per set, become the size of each set and the size of each intersection.
//!
//! - An intersection is an exact pattern of membership: a row in sets A and B only counts for "A and B", not for
//!   "A" (the exclusive counts of UpSet plots). Rows in no set are left out.
//! - Intersections are sorted by size, largest first; equal sizes by the number of sets they join, fewest first,
//!   then by their sets in set order. At most [`MAX_INTERSECTIONS`] are kept (the largest).
//! - A set's size is the number of its rows, whatever other sets they are in.

/// The most intersections an UpSet plot shows.
pub const MAX_INTERSECTIONS: usize = 40;

/// One intersection: which sets (by index) and how many rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intersection {
    pub sets: Vec<usize>,
    pub count: usize,
}

/// The counts of an UpSet plot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpSet {
    pub set_sizes: Vec<usize>,
    pub intersections: Vec<Intersection>,
    /// The intersections left out beyond [`MAX_INTERSECTIONS`].
    pub dropped: usize,
}

/// Counts the rows of `members` (`members[row][set]`) for `sets` sets; a row shorter than `sets` is not in the
/// sets it lacks.
pub fn count(members: &[Vec<bool>], sets: usize) -> UpSet {
    let mut set_sizes = vec![0; sets];
    let mut found: Vec<Intersection> = Vec::new();
    for row in members {
        let pattern: Vec<usize> = (0..sets).filter(|&s| row.get(s).copied().unwrap_or(false)).collect();
        if pattern.is_empty() {
            continue;
        }
        for &s in &pattern {
            set_sizes[s] += 1;
        }
        match found.iter_mut().find(|i| i.sets == pattern) {
            Some(i) => i.count += 1,
            None => found.push(Intersection { sets: pattern, count: 1 }),
        }
    }
    found.sort_by(|a, b| b.count.cmp(&a.count).then(a.sets.len().cmp(&b.sets.len())).then(a.sets.cmp(&b.sets)));
    let dropped = found.len().saturating_sub(MAX_INTERSECTIONS);
    found.truncate(MAX_INTERSECTIONS);
    UpSet { set_sizes, intersections: found, dropped }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(v: &[&[u8]]) -> Vec<Vec<bool>> {
        v.iter().map(|r| r.iter().map(|&x| x != 0).collect()).collect()
    }

    #[test]
    fn exclusive_intersections_sorted_by_size() {
        let m = rows(&[&[1, 0, 0], &[1, 1, 0], &[1, 1, 0], &[0, 1, 0], &[0, 0, 0], &[1, 1, 1], &[0, 0, 1], &[1, 0, 0]]);
        let u = count(&m, 3);
        assert_eq!(u.set_sizes, [5, 4, 2]);
        let got: Vec<(Vec<usize>, usize)> = u.intersections.iter().map(|i| (i.sets.clone(), i.count)).collect();
        assert_eq!(got, [(vec![0], 2), (vec![0, 1], 2), (vec![1], 1), (vec![2], 1), (vec![0, 1, 2], 1)]);
        assert_eq!(u.dropped, 0);
        // Every row in a set is counted once among the intersections.
        assert_eq!(u.intersections.iter().map(|i| i.count).sum::<usize>(), 7);
    }

    #[test]
    fn short_rows_and_many_patterns() {
        let u = count(&rows(&[&[1], &[0, 1, 1]]), 3);
        assert_eq!(u.set_sizes, [1, 1, 1]);
        assert_eq!(u.intersections.len(), 2);
        // 6 sets give up to 63 patterns: only the largest 40 are kept.
        let all: Vec<Vec<bool>> = (1u32..64).map(|p| (0..6).map(|s| p >> s & 1 == 1).collect()).collect();
        let u = count(&all, 6);
        assert_eq!((u.intersections.len(), u.dropped), (MAX_INTERSECTIONS, 23));
        assert!(u.intersections[0].sets.len() == 1, "ties: fewest sets first");
        assert!(count(&[], 2).intersections.is_empty());
    }
}
