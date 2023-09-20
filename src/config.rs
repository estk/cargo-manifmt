/// The config file for formatting toml after sorting.
///
/// Use the `FromStr` to create a config from a string.
///
/// ## Example
/// ```
/// let input = "trailing_comma = true\ncrlf = true";
/// let config = input.parse::<Config>().unwrap();
/// assert!(config.trailing_comma);
/// assert!(config.crlf);
/// ```
#[derive(serde::Deserialize)]
pub struct Config {
    /// Use trailing comma where possible.
    ///
    /// Defaults to `false`.
    #[serde(default)]
    pub always_trailing_comma: bool,

    /// Use trailing comma for multi-line arrays.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_as_true")]
    pub multiline_trailing_comma: bool,

    /// Use space around equal sign for table key values.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_as_true")]
    pub space_around_eq: bool,

    /// Omit whitespace padding inside single-line arrays.
    ///
    /// Defaults to `false`.
    #[serde(default)]
    pub compact_arrays: bool,

    /// Omit whitespace padding inside inline tables.
    ///
    /// Defaults to `false`.
    #[serde(default)]
    pub compact_inline_tables: bool,

    /// Add trailing newline to the source.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_as_true")]
    pub trailing_newline: bool,

    /// Are newlines allowed between key value pairs in a table.
    ///
    /// This must be true for the `--grouped` flag to be used.
    /// Defaults to `true`.
    #[serde(default = "default_as_true")]
    pub key_value_newlines: bool,

    /// The maximum amount of consecutive blank lines allowed.
    ///
    /// Defaults to `1`.
    #[serde(default = "default_as_one")]
    pub allowed_blank_lines: usize,

    // NOTE: this is only used in main, fmt doesn't set the line endings
    /// Use CRLF line endings
    ///
    /// Defaults to `false`.
    #[serde(default)]
    pub crlf: bool,

    /// The user specified ordering of tables in a document.
    ///
    /// All unspecified tables will come after these.
    #[serde(default = "Config::default_table_order")]
    pub table_order: Vec<String>,

    #[serde(default)]
    pub workspace_dependency_grouping: Option<WSDependencyGrouping>,
}

fn default_as_true() -> bool {
    true
}

fn default_as_one() -> usize {
    1
}

const DEFAULT_TABLE_ORDER: &[&str] = &[
    "package",
    "lib",
    "bin",
    "features",
    "dependencies",
    "build-dependencies",
    "dev-dependencies",
];

#[derive(serde::Deserialize)]
pub enum WSDependencyGrouping {
    Top,
    Bottom,
}

impl Config {
    pub fn serde_default() -> Self {
        toml::from_str("").unwrap()
    }
    fn default_table_order() -> Vec<String> {
        DEFAULT_TABLE_ORDER.iter().map(ToString::to_string).collect()
    }

    // Used in testing and fuzzing
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        toml::from_str("").unwrap()
    }
}
