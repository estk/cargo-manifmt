use std::{
    env,
    fmt::Display,
    fs::{self, read_to_string, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
pub(crate) use config::Config;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use toml_edit::{Document, Item};

mod config;
mod sort;

type IoResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Returns non-zero exit code if Cargo.toml is unsorted, overrides default behavior
    #[arg(short, long)]
    check: bool,

    /// Prints Cargo.toml, lexically sorted, to stdout
    #[arg(short, long)]
    // todo .conflicts_with("check")
    print: bool,

    /// Checks every crate in a workspace
    #[arg(short, long)]
    workspace: bool,

    /// List the order tables should be written out (--order package,dependencies,features)
    #[arg(short, long)]
    order: Option<Vec<String>>,
}

fn main() {
    run().unwrap_or_else(|e| {
        write_red("error: ", e).unwrap();
        std::process::exit(1);
    })
}

fn run() -> IoResult<()> {
    let raw_args = std::env::args_os();
    let filtered_args = raw_args.filter(|x| x != "sort");

    let args: Args = Parser::parse_from(filtered_args);
    let cwd =
        env::current_dir().map_err(|e| format!("no current directory found: {}", e))?;
    let cwd_str = cwd.to_string_lossy();

    let cf = fs::File::open(cwd.join("tomlfmt.toml"))
        .or_else(|_| fs::File::open(".tomlfmt.toml"));

    let mut config = if let Ok(mut f) = cf {
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        toml::from_str(&s)?
    } else {
        Config::serde_default()
    };

    if let Some(ordering) = &args.order {
        config.table_order = ordering.into_iter().map(|s| s.to_string()).collect();
    }
    let member_paths = find_members(cwd_str.to_string())?;
    let members_sorted: IoResult<Vec<bool>> =
        member_paths.iter().map(|path| check_toml(path, &args, &config)).collect();
    let all_sorted = members_sorted?.iter().all(|x| *x);

    if all_sorted {
        std::process::exit(0)
    } else {
        std::process::exit(1)
    }
}

fn find_members(dir: String) -> IoResult<Vec<String>> {
    let mut filtered_matches = vec![dir.clone()];

    let mut path = PathBuf::from(&dir);
    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    let raw_toml = read_to_string(&path)
        .map_err(|_| format!("no file found at: {}", path.display()))?;

    let toml = raw_toml.parse::<Document>()?;
    let workspace = &toml["workspace"];
    if let Item::Table(ws) = workspace {
        // The workspace excludes, used to filter members by
        let excludes: Vec<&str> = ws
            .get("exclude")
            .and_then(Item::as_array)
            .into_iter()
            .flat_map(|a| a.iter())
            .flat_map(|s| s.as_str())
            .collect();
        for member in ws
            .get("members")
            .and_then(Item::as_array)
            .into_iter()
            .flat_map(|arr| arr.iter())
            .flat_map(|s| s.as_str())
            .filter(|s| !excludes.contains(s))
        {
            // TODO: a better test wether to glob?
            if member.contains('*') || member.contains('?') {
                'globs: for entry in glob::glob(&format!("{}/{}", &dir, member))
                    .unwrap_or_else(|e| {
                        write_red("error: ", format!("Glob failed: {}", e)).unwrap();
                        std::process::exit(1);
                    })
                {
                    let path = entry?;

                    // The `check_toml` function expects only folders that it appends
                    // `Cargo.toml` onto
                    if path.is_file() {
                        continue;
                    }

                    // Since the glob function gives us actual paths we need to only
                    // check if the relevant parts match so we can't just do
                    // `excludes.contains(..)`
                    let path_str = path.to_string_lossy();
                    for excl in &excludes {
                        if path_str.ends_with(excl) {
                            continue 'globs;
                        }
                    }

                    filtered_matches.push(path.display().to_string());
                }
            } else {
                filtered_matches.push(format!("{}/{}", dir, member));
            }
        }
    }
    Ok(filtered_matches)
}

fn check_toml(path: &str, args: &Args, config: &Config) -> IoResult<bool> {
    let mut path = PathBuf::from(path);
    if path.extension().is_none() {
        path.push("Cargo.toml");
    }

    let krate = path.components().nth_back(1).ok_or("No crate folder found")?.as_os_str();

    write_green("Checking ", format!("{}...", krate.to_string_lossy()))?;

    let toml_raw = read_to_string(&path)
        .map_err(|_| format!("No file found at: {}", path.display()))?;

    let sorted = sort::sort_toml(&toml_raw, sort::MATCHER, false, &config.table_order);
    let mut sorted_str = sorted.to_string();
    let is_sorted = toml_raw == sorted_str;

    if config.crlf && !sorted_str.contains("\r\n") {
        sorted_str = sorted_str.replace('\n', "\r\n")
    }

    if args.print {
        print!("{}", sorted_str);
        return Ok(true);
    }

    if args.check {
        if !is_sorted {
            write_red(
                "error: ",
                format!("Dependencies for {} are not sorted", krate.to_string_lossy()),
            )?;
        }

        return Ok(is_sorted);
    }

    write_file(&path, &sorted_str)?;
    write_green(
        "Finished: ",
        format!("Cargo.toml for {:?} has been rewritten", krate.to_string_lossy()),
    )?;

    Ok(true)
}

fn write_red<S: Display>(highlight: &str, msg: S) -> IoResult<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(stderr, "{}", highlight)?;
    stderr.reset()?;
    writeln!(stderr, "{}", msg).map_err(Into::into)
}

fn write_green<S: Display>(highlight: &str, msg: S) -> IoResult<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    write!(stdout, "{}", highlight)?;
    stdout.reset()?;
    writeln!(stdout, "{}", msg).map_err(Into::into)
}

fn write_file<P: AsRef<Path>>(path: P, toml: &str) -> IoResult<()> {
    let mut fd = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
    write!(fd, "{}", toml).map_err(Into::into)
}
