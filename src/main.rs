use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
struct Cli {
    path: std::path::PathBuf,
}

#[derive(Debug)]
struct CustomError(String);

fn main() -> Result<()> {
    let args = Cli::parse();

    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("could not read file `{}`", args.path.display()))?;

    let warnings = warnsum::find_warnings(&content)?;
    let names = warnsum::count_warning_types(&warnings);
    let files = warnsum::count_warning_files(&warnings);
    let directories = warnsum::count_warning_directories(&warnings);

    println!("Warnings:");
    println!("{}", warnsum::make_warning_counts(&names));
    println!("\nFiles:");
    println!("{}", warnsum::make_warning_counts(&files));
    println!("\nDirectories:");
    println!("{}", warnsum::make_warning_counts(&directories));

    Ok(())
}
