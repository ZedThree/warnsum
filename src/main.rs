// warnsum: summarise compiler warnings
// Copyright (C) 2023 Peter Hill
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use anyhow::{Context, Result};
use clap::Parser;
use warnsum::WarningCollection;

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

    let warnings = WarningCollection::new(&content, args.keyword_len, &args.ignore);

    println!("{warnings:.width$}", width = &args.top_n);

    Ok(())
}
