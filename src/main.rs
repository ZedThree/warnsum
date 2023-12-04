use anyhow::{Context, Result};
use clap::Parser;

/// Summarise compiler warnings from log file
#[derive(Parser, Debug)]
struct Cli {
    /// Path to log file
    path: std::path::PathBuf,

    /// Top N items to display in each category
    #[arg(short = 'n', default_value_t = 10)]
    top_n: usize,

    /// Length of interesting keywords
    #[arg(short, default_value_t = 5)]
    keyword_len: usize,

    /// Keywords to ignore from warnings
    #[arg(short, long, num_args = 1.., value_delimiter = ' ')]
    ignore: Vec<String>,
}

#[derive(Debug)]
struct CustomError(String);

fn main() -> Result<()> {
    let args = Cli::parse();

    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("could not read file `{}`", args.path.display()))?;

    let warnings = warnsum::find_warnings(&content, args.keyword_len, &args.ignore)?;
    let names = warnsum::count_warning_types(&warnings);
    let files = warnsum::count_warning_files(&warnings);
    let directories = warnsum::count_warning_directories(&warnings);
    let keywords = warnsum::count_warning_keywords(&warnings);

    println!("Warnings:");
    println!("{}", warnsum::make_warning_counts(&names, 0, false));
    println!("\nFiles:");
    println!("{}", warnsum::make_warning_counts(&files, args.top_n, true));
    println!("\nDirectories:");
    println!(
        "{}",
        warnsum::make_warning_counts(&directories, args.top_n, true)
    );
    println!("\nKeywords:");
    println!(
        "{}",
        warnsum::make_warning_counts(&keywords, args.top_n, true)
    );

    Ok(())
}
