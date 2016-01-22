//! An implementation of [tldr](https://github.com/tldr-pages/tldr) in Rust.

#[macro_use] extern crate log;
#[cfg(feature = "logging")]extern crate env_logger;
extern crate docopt;
extern crate ansi_term;
extern crate flate2;
extern crate tar;
extern crate curl;
extern crate rustc_serialize;
extern crate time;

use std::io::BufReader;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process;

use docopt::Docopt;

mod types;
mod tokenizer;
mod formatter;
mod cache;
mod error;

use tokenizer::Tokenizer;
use cache::Cache;
use error::TldrError::{UpdateError, CacheError};
use formatter::print_lines;
use types::OsType;


const NAME: &'static str = "tldr-rs";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const USAGE: &'static str = "
Usage:

    tldr <command>
    tldr [options]

Options:

    -h --help           Show this screen
    -v --version        Show version information
    -l --list           List all commands in the cache
    -f --render <file>  Render a specific markdown file
    -o --os <type>      Override the operating system [linux, osx, sunos]
    -u --update         Update the local cache
    -c --clear-cache    Clear the local cache

Examples:

    $ tldr tar
    $ tldr --list

To control the cache:

    $ tldr --update
    $ tldr --clear-cache

To render a local file (for testing):

    $ tldr --render /path/to/file.md
";
const ARCHIVE_URL: &'static str = "https://github.com/tldr-pages/tldr/archive/master.tar.gz";
const MAX_CACHE_AGE: i64 = 2592000; // 30 days


#[derive(Debug, RustcDecodable)]
struct Args {
    arg_command: Option<String>,
    flag_help: bool,
    flag_version: bool,
    flag_list: bool,
    flag_render: Option<String>,
    flag_os: Option<OsType>,
    flag_update: bool,
    flag_clear_cache: bool,
}


/// Print page by path
fn print_page(path: &Path) -> Result<(), String> {
    // Open file
    let file = try!(
        File::open(path).map_err(|msg| format!("Could not open file: {}", msg))
    );
    let reader = BufReader::new(file);

    // Create tokenizer and print output
    let mut tokenizer = Tokenizer::new(reader);
    print_lines(&mut tokenizer);

    Ok(())
}


#[cfg(feature = "logging")]
fn init_log() {
    env_logger::init().unwrap();
}

#[cfg(not(feature = "logging"))]
fn init_log() { }


#[cfg(target_os = "linux")]
fn get_os() -> OsType { OsType::Linux }

#[cfg(target_os = "macos")]
fn get_os() -> OsType { OsType::OsX }

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_os() -> OsType { OsType::Other }


fn main() {

    // Initialize logger
    init_log();

    // Parse arguments
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    // Show version and exit
    if args.flag_version {
        println!("{} v{}", NAME, VERSION);
        process::exit(0);
    }

    // Initialize cache
    let os: OsType = get_os();
    let cache = Cache::new(ARCHIVE_URL, os);

    // Clear cache, pass through
    if args.flag_clear_cache {
        cache.clear().unwrap_or_else(|e| {
            match e {
                UpdateError(msg) | CacheError(msg) => println!("Could not delete cache: {}", msg),
            };
            process::exit(1);
        });
        println!("Successfully deleted cache.");
    }

    // Update cache, pass through
    if args.flag_update {
        cache.update().unwrap_or_else(|e| {
            match e {
                UpdateError(msg) | CacheError(msg) => println!("Could not update cache: {}", msg),
            };
            process::exit(1);
        });
        println!("Successfully updated cache.");
    }

    // Render local file and exit
    if let Some(file) = args.flag_render {
        let path = PathBuf::from(file);
        if let Err(msg) = print_page(&path) {
            println!("{}", msg);
            process::exit(1);
        } else {
            process::exit(0);
        };
    }

    // List cached commands and exit
    if args.flag_list {
        println!("Flag --list not yet implemented.");
        process::exit(1);
    }

    // Show command from cache
    if let Some(command) = args.arg_command {

        // Check cache
        if !args.flag_update {
            match cache.last_update() {
                Some(ago) if ago > MAX_CACHE_AGE => {
                    println!("Cache wasn't updated in {} days.", MAX_CACHE_AGE / 24 / 3600);
                    println!("You should probably run `tldr --update` soon.");
                },
                None => {
                    println!("Cache not found. Please run `tldr --update`.");
                    process::exit(1);
                },
                _ => {},
            }
        }

        // Search for command in cache
        if let Some(path) = cache.find_page(&command) {
            if let Err(msg) = print_page(&path) {
                println!("{}", msg);
                process::exit(1);
            } else {
                process::exit(0);
            }
        } else {
            println!("Page {} not found in cache", &command);
            println!("Try updating with `tldr --update`, or submit a pull request to:");
            println!("https://github.com/tldr-pages/tldr");
            process::exit(1);
        }
    }

    // Some flags can be run without a command.
    if !(args.flag_update || args.flag_clear_cache) {
        println!("{}", USAGE);
        process::exit(1);
    }
}


#[cfg(test)]
mod test {
    use types::LineType;

    #[test]
    fn test_linetype_from_str() {
        assert_eq!(LineType::from(""), LineType::Empty);
        assert_eq!(LineType::from(" \n \r"), LineType::Empty);
        assert_eq!(LineType::from("# Hello there"), LineType::Title("Hello there".into()));
        assert_eq!(LineType::from("> tis a description \n"), LineType::Description("tis a description".into()));
        assert_eq!(LineType::from("- some command"), LineType::ExampleText("some command".into()));
        assert_eq!(LineType::from("`$ cargo run`"), LineType::ExampleCode("$ cargo run".into()));
        assert_eq!(LineType::from("`$ cargo run"), LineType::Other("`$ cargo run".into()));
        assert_eq!(LineType::from("jklö"), LineType::Other("jklö".into()));
    }
}
