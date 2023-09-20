#![cfg(test)]
use std::fs;

use super::Matcher;

const MATCHER: Matcher<'_> = Matcher {
    heading: &["dependencies", "dev-dependencies", "build-dependencies"],
    heading_key: &[
        ("workspace", "members"),
        ("workspace", "exclude"),
        ("workspace", "dependencies"),
    ],
};

#[test]
fn toml_edit_check() {
    let input = fs::read_to_string("examp/workspace.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, false, &[]);
    assert_ne!(input, sorted.to_string());
    // println!("{}", sorted.to_string_in_original_order());
}

#[test]
fn grouped_check() {
    let input = fs::read_to_string("examp/ruma.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    assert_ne!(input, sorted.to_string());
    // println!("{}", sorted.to_string());
}

#[test]
fn sort_correct() {
    let input = fs::read_to_string("examp/right.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    #[cfg(target_os = "windows")]
    assert_eq!(input.replace("\r\n", "\n"), sorted.to_string().replace("\r\n", "\n"));
    #[cfg(not(target_os = "windows"))]
    assert_eq!(input, sorted.to_string());
    // println!("{}", sorted.to_string());
}

#[test]
fn sort_tables() {
    let input = fs::read_to_string("examp/fend.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    assert_ne!(input, sorted.to_string());
    // println!("{}", sorted.to_string_in_original_order());
}

#[test]
fn sort_devfirst() {
    let input = fs::read_to_string("examp/reorder.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    #[cfg(target_os = "windows")]
    assert_eq!(input.replace("\r\n", "\n"), sorted.to_string().replace("\r\n", "\n"));
    #[cfg(not(target_os = "windows"))]
    assert_eq!(input, sorted.to_string());
    // println!("{}", sorted.to_string_in_original_order());

    let input = fs::read_to_string("examp/noreorder.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    #[cfg(target_os = "windows")]
    assert_eq!(input.replace("\r\n", "\n"), sorted.to_string().replace("\r\n", "\n"));
    #[cfg(not(target_os = "windows"))]
    assert_eq!(input, sorted.to_string());
    // println!("{}", sorted.to_string_in_original_order());
}

#[test]
fn reorder() {
    let input = fs::read_to_string("examp/clippy.toml").unwrap();
    let sorted = super::sort_toml(
        &input,
        MATCHER,
        true,
        &[
            "package".to_owned(),
            "features".to_owned(),
            "dependencies".to_owned(),
            "build-dependencies".to_owned(),
            "dev-dependencies".to_owned(),
        ],
    );
    assert_ne!(input, sorted.to_string());
}

#[test]
fn workspace_dependencies_check() {
    let input = fs::read_to_string("examp/workspace_dep.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, false, &[]);
    assert_ne!(input, sorted.to_string());
    println!("{}", sorted.to_string());
}
