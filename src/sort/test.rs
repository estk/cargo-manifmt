#![cfg(test)]
use std::fs;

use pretty_assertions::{assert_eq, assert_ne};

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
    let input = fs::read_to_string("fixtures/workspace.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, false, &[]);
    assert_ne!(input, sorted.to_string());
}

#[test]
fn grouped_check() {
    let input = fs::read_to_string("fixtures/ruma.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    assert_ne!(input, sorted.to_string());
}

#[test]
fn sort_correct() {
    let input = fs::read_to_string("fixtures/right.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    assert_eq!(input.replace("\r\n", "\n"), sorted.to_string());
}

#[test]
fn sort_tables() {
    let input = fs::read_to_string("fixtures/fend.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    assert_ne!(input, sorted.to_string());
}

#[test]
fn sort_devfirst() {
    let input = fs::read_to_string("fixtures/reorder.toml").unwrap();

    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    let sorted = sorted.to_string();
    assert_eq!(input.replace("\r\n", "\n"), sorted.to_string());

    let input = fs::read_to_string("fixtures/noreorder.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, true, &[]);
    assert_eq!(input.replace("\r\n", "\n"), sorted.to_string());
}

#[test]
fn reorder() {
    let input = fs::read_to_string("fixtures/clippy.toml").unwrap();
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
    let input = fs::read_to_string("fixtures/workspace_dep.toml").unwrap();
    let sorted = super::sort_toml(&input, MATCHER, false, &[]);
    assert_ne!(input, sorted.to_string());
}
