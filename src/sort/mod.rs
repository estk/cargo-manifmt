use std::{cmp::Ordering, collections::BTreeMap, iter::FromIterator};

use toml_edit::{Array, Document, Item, Table, TableLike, Value};

mod test;

/// Each `Matcher` field when matched to a heading or key token
/// will be matched with `.contains()`.
pub struct Matcher<'a> {
    /// Toml headings with braces `[heading]`.
    pub heading: &'a [&'a str],
    /// Toml heading with braces `[heading]` and the key
    /// of the array to sort.
    pub heading_key: &'a [(&'a str, &'a str)],
}

pub const MATCHER: Matcher<'_> = Matcher {
    heading: &["dependencies", "dev-dependencies", "build-dependencies"],
    heading_key: &[
        ("workspace", "members"),
        ("workspace", "exclude"),
        ("workspace", "dependencies"),
    ],
};

/// A state machine to track collection of headings.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Heading {
    /// After collecting heading segments we recurse into another table.
    Next(Vec<String>),
    /// We have found a completed heading.
    ///
    /// The the heading we are processing has key value pairs.
    Complete(Vec<String>),
}

fn gather_headings(table: &Table, keys: &mut Vec<Heading>, depth: usize) {
    if table.is_empty() && !table.is_implicit() {
        let next = match keys.pop().unwrap() {
            Heading::Next(segs) => Heading::Complete(segs),
            comp => comp,
        };
        keys.push(next);
    }
    for (head, item) in table.iter() {
        match item {
            Item::Value(_) => {
                if keys.last().map_or(false, |h| matches!(h, Heading::Complete(_))) {
                    continue;
                }
                let next = match keys.pop().unwrap() {
                    Heading::Next(segs) => Heading::Complete(segs),
                    _complete => unreachable!("the above if check prevents this"),
                };
                keys.push(next);
                continue;
            }
            Item::Table(table) => {
                let next = match keys.pop().unwrap() {
                    Heading::Next(mut segs) => {
                        segs.push(head.into());
                        Heading::Next(segs)
                    }
                    // This happens when
                    //
                    // [heading]       // transitioning from here to
                    // [heading.segs]  // here
                    Heading::Complete(segs) => {
                        let take = depth.max(1);
                        let mut next = segs[..take].to_vec();
                        next.push(head.into());
                        keys.push(Heading::Complete(segs));
                        Heading::Next(next)
                    }
                };
                keys.push(next);
                gather_headings(table, keys, depth + 1);
            }
            Item::ArrayOfTables(_arr) => unreachable!("no [[heading]] are sorted"),
            Item::None => unreachable!("an empty table will not be sorted"),
        }
    }
}

fn sort_by_group(table: &mut Table) {
    let table_clone = table.clone();
    table.clear();
    let mut groups = BTreeMap::new();
    let mut curr = 0;
    for (idx, (k, v)) in table_clone.iter().enumerate() {
        let decor = table.key_decor(k);

        if decor.map_or(0, count_blank_lines) > 0 {
            groups.entry(idx).or_insert_with(|| vec![(k, v)]);
            curr = idx;
        } else {
            groups.entry(curr).or_default().push((k, v));
        }
    }

    for (_, mut group) in groups {
        group.sort_by_key(|x| x.0);

        for (k, v) in group {
            table.insert(k, v.clone());

            // Transfer key decor from cloned table to modified table. Apparently
            // inserting v.clone() does not work like that.
            if let (Some(decor_mut), Some(decor)) =
                (table.key_decor_mut(k), table_clone.key_decor(k))
            {
                if let Some(prefix) = decor.prefix() {
                    decor_mut.set_prefix(prefix.clone());
                }

                if let Some(suffix) = decor.suffix() {
                    decor_mut.set_suffix(suffix.clone());
                }
            }
        }
    }
}

fn sort_array(arr: &mut Array) {
    let mut all_strings = true;
    let mut arr_copy = arr.iter().cloned().collect::<Vec<_>>();
    arr_copy.sort_by(|a, b| match (a, b) {
        (Value::String(a), Value::String(b)) => a.value().cmp(b.value()),
        _ => {
            all_strings = false;
            Ordering::Equal
        }
    });
    if all_strings {
        *arr = Array::from_iter(arr_copy);
    }
}
/// check if the dependency value indicates that it is a workspace dep
fn is_ws_dep(t: &dyn TableLike) -> bool {
    t.get("workspace").and_then(|ws| ws.as_bool()).is_some()
}
fn is_onekey(t: &dyn TableLike) -> bool { t.len() == 1 }
fn is_git(t: &dyn TableLike) -> bool { t.contains_key("git") }
fn is_path(t: &dyn TableLike) -> bool { t.contains_key("path") }

/// Returns a sorted toml `Document`.
pub fn sort_toml(
    input: &str,
    matcher: Matcher<'_>,
    group: bool,
    ordering: &[String],
) -> Document {
    let mut ordering = ordering.to_owned();
    let mut toml = input.parse::<Document>().unwrap();

    // This takes care of `[workspace] members = [...]`
    // and the [workspace.dependencies] table
    for (heading, key) in matcher.heading_key {
        // Since this `&mut toml[&heading]` is like
        // `SomeMap.entry(key).or_insert(Item::None)` we only want to do it if we
        // know the heading is there already
        if let Some((_k, Item::Table(table))) =
            toml.as_table_mut().get_key_value_mut(heading)
        {
            match table.get_key_value_mut(key) {
                Some((_, Item::Value(Value::Array(arr)))) => {
                    sort_array(arr);
                }
                Some((_, Item::Table(tab))) => {
                    if key.ends_with("dependencies") {
                        sort_deps(tab);
                    } else {
                        tab.sort_values();
                    }
                }
                _ => {}
            }
        }
    }

    let mut first_table = None;
    let mut heading_order: BTreeMap<_, Vec<Heading>> = BTreeMap::new();
    for (idx, (head, item)) in toml.as_table_mut().iter_mut().enumerate() {
        if !matcher.heading.contains(&head.display_repr().as_ref()) {
            let head = head.to_owned();

            if !ordering.contains(&head) && !ordering.is_empty() {
                ordering.push(head);
            }
            continue;
        }
        match item {
            Item::Table(table) => {
                if first_table.is_none() {
                    // The root table is always index 0 which we ignore so add 1
                    first_table = Some(idx + 1);
                }
                let headings = heading_order.entry((idx, head.to_string())).or_default();
                // Push a `Heading::Complete` here incase the tables are ordered
                // [heading.segs]
                // [heading]
                // It will just be ignored if not the case
                headings.push(Heading::Complete(vec![head.to_string()]));

                gather_headings(table, headings, 1);
                headings.sort();
                if head.to_string().ends_with("dependencies") {
                    sort_deps(table);
                } else if group {
                    sort_by_group(table);
                } else {
                    table.sort_values()
                }
            }
            Item::None => continue,
            _ => unreachable!("Top level toml must be tables"),
        }
    }

    if ordering.is_empty() {
        sort_lexicographical(first_table, &heading_order, &mut toml);
    } else {
        sort_by_ordering(&ordering, &heading_order, &mut toml);
    }

    toml
}

fn sort_lexicographical(
    first_table: Option<usize>,
    heading_order: &BTreeMap<(usize, String), Vec<Heading>>,
    toml: &mut Document,
) {
    // Since the root table is always index 0 we add one
    let first_table_idx = first_table.unwrap_or_default() + 1;
    for (idx, heading) in heading_order.iter().flat_map(|(_, segs)| segs).enumerate() {
        if let Heading::Complete(segs) = heading {
            let mut nested = 0;
            let mut table = Some(toml.as_table_mut());
            for seg in segs {
                nested += 1;
                table = table.and_then(|t| t[seg].as_table_mut());
            }
            // Do not reorder the unsegmented tables
            if nested > 1 {
                if let Some(table) = table {
                    table.set_position(first_table_idx + idx);
                }
            }
        }
    }
}

fn sort_by_ordering(
    ordering: &[String],
    heading_order: &BTreeMap<(usize, String), Vec<Heading>>,
    toml: &mut Document,
) {
    let mut idx = 0;
    for heading in ordering {
        if let Some((_, to_sort_headings)) =
            heading_order.iter().find(|((_, key), _)| key == heading)
        {
            for h in to_sort_headings {
                if let Heading::Complete(segs) = h {
                    let mut table = Some(toml.as_table_mut());
                    for seg in segs {
                        table = table.and_then(|t| t[seg].as_table_mut());
                    }
                    // Do not reorder the unsegmented tables
                    if let Some(table) = table {
                        table.set_position(idx);
                        idx += 1;
                    }
                }
            }
        } else if let Some(tab) = toml.as_table_mut()[heading].as_table_mut() {
            tab.set_position(idx);
            idx += 1;
            walk_tables_set_position(tab, &mut idx)
        } else if let Some(arrtab) = toml.as_table_mut()[heading].as_array_of_tables_mut()
        {
            for tab in arrtab.iter_mut() {
                tab.set_position(idx);
                idx += 1;
                walk_tables_set_position(tab, &mut idx);
            }
        }
    }
}

fn walk_tables_set_position(table: &mut Table, idx: &mut usize) {
    for (_, item) in table.iter_mut() {
        match item {
            Item::Table(tab) => {
                tab.set_position(*idx);
                *idx += 1;
                walk_tables_set_position(tab, idx)
            }
            Item::ArrayOfTables(arr) => {
                for tab in arr.iter_mut() {
                    tab.set_position(*idx);
                    *idx += 1;
                    walk_tables_set_position(tab, idx)
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn count_blank_lines(decor: &toml_edit::Decor) -> usize {
    decor
        .prefix()
        .map_or(Some(""), |s| s.as_str())
        .unwrap_or("")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .count()
}

fn sort_deps(table: &mut Table) {
    use itertools::Itertools;

    let groups: Vec<Vec<String>> = {
        // iterator of meta & key
        let key_meta = table.iter().map(|e| (DepMeta::from_entry(&e), e.0));

        // sorted iter of meta & key (group_by only works when pre-sorted)
        let sorted_by_meta = key_meta.sorted_by_key(|(m, _s)| *m);

        // grouped by meta
        let grouped_by_meta = sorted_by_meta.group_by(|(m, _k)| *m);

        // sort the items in each group, lexically
        let grouped_and_sorted_items = grouped_by_meta.into_iter().map(|(_, group)| {
            let iter = group.map(|(_m, k)| k.to_string()).sorted();
            iter.collect_vec()
        });

        grouped_and_sorted_items.collect()
    };

    for group in groups {
        for k in group {
            let Some(orig_decor) = table.key_decor(&k).map(ToOwned::to_owned) else {
                tracing::warn!("Unable to find key decor for {k} in table");
                continue;
            };
            let Some((k, mut v)) = table.remove_entry(&k) else {
                tracing::warn!("Unable to find entry for {k} in table");
                continue;
            };
            // todo: factor this somewhere else
            // transform single key tables to inline tables
            let mut dotted = false;
            if let Some(t) = v.as_inline_table_mut() {
                dotted = true;
                t.decor_mut().clear();
                // avoid any extra spaces from when it was a normal table
                for (mut k, _) in t.iter_mut() {
                    k.decor_mut().clear();
                }
                if t.len() == 1 {
                    t.set_dotted(true);
                }
            }
            table.insert(&k, v);
            let d = table.key_decor_mut(&k).unwrap();

            if let Some(pfx) = orig_decor.prefix() {
                d.set_prefix(pfx.to_owned())
            }

            if let Some(od) = orig_decor.suffix() {
                if !dotted {
                    d.set_suffix(od.to_owned())
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Hash, Copy, Clone)]
enum DepMeta {
    Path,
    Git,
    NormalTable,
    Other,
    String,
    Ws,
    WsOneKey,
}
impl DepMeta {
    fn from_entry((_, i): &(&str, &Item)) -> Self {
        if matches!(i, Item::Value(Value::String(_))) {
            return Self::String;
        }

        if let Some(t) = i.as_table_like() {
            if is_git(t) {
                Self::Git
            } else if is_path(t) {
                Self::Path
            } else if is_ws_dep(t) {
                if is_onekey(t) { Self::WsOneKey } else { Self::Ws }
            } else {
                Self::NormalTable
            }
        } else {
            Self::Other
        }
    }
}
