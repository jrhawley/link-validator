use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use clap::{crate_authors, crate_description, crate_version, App, Arg};
use comrak::{
    nodes::{AstNode, NodeValue},
    parse_document, Arena, ComrakExtensionOptions, ComrakOptions,
};
use percent_encoding::percent_decode;
use url::Url;

fn main() {
    let matches = App::new("mlv")
        .about(crate_description!())
        .author(crate_authors!())
        .version(crate_version!())
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
        match src.extension() {
            Some(ext) => {
                let ext_str = String::from(ext.to_str().unwrap());
                match ext_str.as_str() {
                    "md" | "MD" | "markdown" => match validate_file(&src) {
                        Ok(_) => {}
                        Err(_) => {}
                    },
                    _ => {
                        eprintln!(
                            "`{}` does not appear to me a Markdown file. Skipping.",
                            src.display()
                        );
                    }
                }
            }
            _ => {
                eprintln!(
                    "`{}` does not appear to me a Markdown file. Skipping.",
                    src.display()
                );
            }
        }
    } else if src.is_dir() {
        todo!();
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

fn validate_file(file: &Path) -> io::Result<bool> {
    let file_contents = read_markdown(file)?;
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

    // check that each file link exists
    let mut missing_links: Vec<PathBuf> = Vec::new();
    for l in &file_links {
        if !l.exists() {
            missing_links.push(l.clone());
        }
    }
    if missing_links.len() > 0 {
        eprintln!("The following links are not found:");
        for l in missing_links {
            eprintln!("{}", l.display());
        }
    }

    Ok(false)
}
