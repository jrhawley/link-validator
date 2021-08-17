use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg};
use comrak::{
    nodes::{AstNode, NodeValue},
    parse_document, Arena, ComrakExtensionOptions, ComrakOptions,
};
use percent_encoding::percent_decode;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use url::Url;
use walkdir::WalkDir;

fn main() {
    let matches = app_from_crate!()
        .arg(Arg::with_name("src")
            .takes_value(true)
            .required(true)
            .help("Source file or directory to parse. If a directory, validates every Markdown file found within it.")
        )
        .get_matches();

    // check that the input source is a file or directory that exists
    let src = PathBuf::from(matches.value_of("src").unwrap());
    if !src.exists() {
        eprintln!("`{}` not found. Skipping.", src.display());
    }
    if src.is_file() {
        if is_markdown(src.as_path()) {
            let missing_links = get_missing_links(src.as_path());
            if missing_links.len() > 0 {
                eprintln!("The following linked files cannot be found:");
            }
            print_missing(missing_links, src.as_path(), false);
        } else {
            eprintln!(
                "`{}` does not appear to me a Markdown file. Skipping.",
                src.display()
            );
        }
    } else if src.is_dir() {
        let mut any_missing = false;
        for entry in WalkDir::new(&src)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| is_markdown(e.path()))
        {
            let missing_links = get_missing_links(entry.path());
            if (missing_links.len() > 0) && !any_missing {
                any_missing = true;
                eprintln!("The following linked files cannot be found:");
            }
            print_missing(missing_links, entry.path(), true);
        }
    } else {
        eprintln!(
            "`{}` is neither a file nor a directory. Skipping.",
            src.display()
        );
    }
}

/// Convert the markdown document to a string
fn read_markdown(path: &Path) -> io::Result<String> {
    let mut file = File::open(&path)?;
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)?;
    Ok(file_contents)
}

/// Extract all the links from the children within a node
fn extract_links<'a>(node: &'a AstNode<'a>, output: &mut Vec<String>) {
    match node.data.borrow().value {
        NodeValue::Link(ref link) => {
            if let Ok(p) = String::from_utf8(link.url.clone()) {
                output.push(p);
            }
        }
        _ => {
            for child in node.children() {
                extract_links(child, output);
            }
        }
    }
}

fn get_missing_links(file: &Path) -> Vec<PathBuf> {
    let file_contents = read_markdown(file).unwrap();
    let arena = Arena::new();
    let opts = ComrakOptions {
        extension: ComrakExtensionOptions {
            table: true,
            autolink: true,
            ..ComrakExtensionOptions::default()
        },
        ..ComrakOptions::default()
    };
    let root = parse_document(&arena, &file_contents, &opts);

    // keep track of all the links in the file
    let mut links: Vec<String> = Vec::new();
    let mut file_links: Vec<PathBuf> = Vec::new();

    // iterate through all the nodes to collect links
    for node in root.children() {
        extract_links(node, &mut links);
    }

    // for each link, determine if it's a URL or a local file
    for l in &links {
        // if it's a URL, ignore it
        if Url::parse(l).is_err() {
            // if it's not a URL, decode the percentage-encoded characters
            match percent_decode(l.as_bytes()).decode_utf8() {
                Ok(decoded) => {
                    let p = PathBuf::from(decoded.to_string());
                    file_links.push(p);
                }
                Err(e) => {
                    eprintln!("Error decoding the following path: {}", l);
                    eprintln!("The following error was produced: {}", e);
                }
            }
        }
    }

    // check that each path is absolute
    // relative paths should be defined relative to the file they come from
    let base_dir = match file.parent() {
        Some(dir) => dir.to_path_buf(),
        None => PathBuf::new(),
    };
    for l in file_links.iter_mut() {
        if l.is_relative() {
            // can guarantee the unwrap because of the file name validation from before
            let new_file = base_dir.join(l.as_path());
            *l = new_file;
        }
    }

    // check that each file link exists
    let mut missing_links: Vec<PathBuf> = Vec::new();
    for l in &file_links {
        if !l.exists() {
            missing_links.push(l.clone());
        }
    }

    missing_links
}

/// Check if the file appears to be a Markdown text file
fn is_markdown(file: &Path) -> bool {
    match file.extension() {
        Some(ext) => {
            let ext_str = String::from(ext.to_str().unwrap());
            match ext_str.as_str() {
                "md" | "MD" | "markdown" => true,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Print the missing links associated with the source file
fn print_missing(missing: Vec<PathBuf>, file: &Path, print_filename: bool) {
    if print_filename {
        for m in missing {
            eprintln!("");
            writeln_colour(file.to_str().unwrap(), Color::Magenta);
            writeln_colour(m.to_str().unwrap(), Color::White);
        }
    } else {
        for m in missing {
            writeln_colour(m.to_str().unwrap(), Color::White);
        }
    }
}

/// Print the text in colour
fn write_colour(s: &str, colour: Color) -> io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(colour)))?;
    write!(&mut stdout, "{}", s)
}

/// Print the text in colour
fn writeln_colour(s: &str, colour: Color) -> io::Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(colour)))?;
    writeln!(&mut stdout, "{}", s)
}
