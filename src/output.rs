//! Functions for printing pages to the terminal

use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
};

use crate::{
    cache::PageLookupResult,
    config::{Config, StyleConfig},
    error::TealdeerError::WriteError,
    formatter::{highlight_lines, PageSnippet},
    line_iterator::LineIterator,
};

/// Print page by path
pub fn print_page(
    page: &PageLookupResult,
    enable_markdown: bool,
    config: &Config,
) -> Result<(), String> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for path in page.paths() {
        let file = File::open(path).map_err(|msg| format!("Could not open file: {}", msg))?;
        let reader = BufReader::new(file);

        if enable_markdown {
            // Print the raw markdown of the file.
            for line in reader.lines() {
                writeln!(handle, "{}", line.unwrap())
                    .map_err(|_| "Could not write to stdout".to_string())?;
            }
        } else {
            let mut process_snippet = |snip: PageSnippet<'_>| {
                if snip.is_empty() {
                    Ok(())
                } else {
                    print_snippet(&mut handle, snip, &config.style)
                        .map_err(|e| WriteError(e.to_string()))
                }
            };
            highlight_lines(
                LineIterator::new(reader),
                &mut process_snippet,
                !config.display.compact,
            )
            .map_err(|e| format!("Could not write to stdout: {}", e.message()))?;
        };
    }

    handle
        .flush()
        .map_err(|_| "Could not flush stdout".to_string())?;

    Ok(())
}

fn print_snippet(
    writer: &mut impl Write,
    snip: PageSnippet<'_>,
    style: &StyleConfig,
) -> Result<(), io::Error> {
    use PageSnippet::*;

    match snip {
        CommandName(s) => write!(writer, "{}", style.command_name.paint(s)),
        Variable(s) => write!(writer, "{}", style.example_variable.paint(s)),
        NormalCode(s) => write!(writer, "{}", style.example_code.paint(s)),
        Description(s) => writeln!(writer, "  {}", style.description.paint(s)),
        Text(s) => writeln!(writer, "  {}", style.example_text.paint(s)),
        Linebreak => writeln!(writer),
    }
}
