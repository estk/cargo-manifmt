use std::{cmp::Ordering, collections::BTreeMap, iter::FromIterator};

use toml_edit::{Array, Document, Item, Table, Value};

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
fn is_ws_dep(item: &Item) -> bool {
    item.as_table_like()
        .and_then(|t| t.get("workspace"))
        .and_then(|ws| ws.as_bool())
        .unwrap_or_default()
}
fn is_onekey(item: &Item) -> bool {
    item.as_table_like().map(|t| t.len() == 1).unwrap_or_default()
}
fn is_git(item: &Item) -> bool {
    item.as_table_like().map(|t| t.contains_key("git")).unwrap_or_default()
}
fn is_path(item: &Item) -> bool {
    item.as_table_like().map(|t| t.contains_key("path")).unwrap_or_default()
}

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

    let gb = table.iter().group_by(DepKind::from_entry);
    let sorted_groups = gb
        .into_iter()
        .into_group_map()
        .into_iter()
        .sorted_by(|(k1, _g1), (k2, _g2)| k1.cmp(k2));

    let mut res = vec![];
    for (_, g) in sorted_groups {
        let mut full_group = vec![];
        for gg in g {
            full_group.extend(gg.into_iter().map(|x| x.0.to_string()))
        }
        full_group.sort();
        res.append(&mut full_group);
    }
    drop(gb);

    for k in res {
        let decor = table.key_decor(&k).unwrap().to_owned();
        let (k, v) = table.remove_entry(&k).unwrap();
        table.insert(&k, v);
        let d = table.key_decor_mut(&k).unwrap();
        if let Some(x) = decor.prefix() {
            d.set_prefix(x.to_owned())
        }
        if let Some(x) = decor.suffix() {
            d.set_suffix(x.to_owned())
        }
    }
}

#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
enum DepKind {
    Path,
    Git,
    Normal,
    Ws,
    WsOneKey,
}
impl DepKind {
    fn from_entry((_, i): &(&str, &Item)) -> Self {
        if is_git(i) {
            Self::Git
        } else if is_path(i) {
            Self::Path
        } else if is_ws_dep(i) {
            if is_onekey(i) {
                Self::WsOneKey
            } else {
                Self::Ws
            }
        } else {
            DepKind::Normal
        }
    }
}
