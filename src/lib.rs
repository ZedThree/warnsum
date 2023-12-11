use core::fmt;
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashMap, env::current_dir, hash::Hash, path::Path, path::PathBuf};

/// A compiler warning
#[derive(Debug, PartialEq, Clone)]
pub struct Warning {
    /// Name of the warning, minus the initial "-W"
    name: String,

    /// File the warning appears in
    file: PathBuf,

    /// Set of potentially interesting keywords from line that raised warning
    keywords: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WarningCollection {
    /// Set of warnings from a whole project
    warnings: Vec<Warning>,

    /// Mapping of warning names to counts
    names: HashMap<String, i16>,

    /// Mapping of filenames to counts
    files: HashMap<PathBuf, i16>,

    /// Mapping of directory names to counts
    directories: HashMap<PathBuf, i16>,

    /// Mapping of keywords to counts
    keywords: HashMap<String, i16>,
}

fn make_keywords<T: AsRef<str>>(
    text: &str,
    keyword_len: usize,
    ignored_keywords: &[T],
) -> Vec<String> {
    lazy_static! {
        static ref WORDS_RE: Regex = Regex::new(r"\b[a-zA-Z_]\w+\b").unwrap();
    }

    WORDS_RE
        .find_iter(text)
        .filter(|mat| mat.as_str().len() >= keyword_len)
        .map(|mat| mat.as_str())
        .filter(|&word| {
            !ignored_keywords
                .iter()
                .any(|ignore| ignore.as_ref() == word)
        })
        .map(|word| word.to_string())
        .collect()
}

fn count_warning_fn<F, T>(warnings: &[Warning], f: F) -> HashMap<T, i16>
where
    F: Fn(&Warning) -> T,
    T: Eq + std::hash::Hash,
{
    let mut result = HashMap::new();

    for warning in warnings {
        let count = result.entry(f(warning)).or_insert(0);
        *count += 1;
    }

    return result;
}

fn count_warning_types(warnings: &[Warning]) -> HashMap<String, i16> {
    count_warning_fn(warnings, |warning| warning.name.clone())
}

fn count_warning_files(warnings: &[Warning]) -> HashMap<PathBuf, i16> {
    count_warning_fn(warnings, |warning| warning.file.clone())
}

fn count_warning_directories(warnings: &[Warning]) -> HashMap<PathBuf, i16> {
    count_warning_fn(warnings, |warning| {
        warning.file.parent().unwrap_or(&warning.file).to_path_buf()
    })
}

fn count_warning_keywords(warnings: &[Warning]) -> HashMap<String, i16> {
    let keywords = warnings
        .iter()
        .map(|warning| &warning.keywords)
        .flatten()
        .collect::<Vec<&String>>();

    let mut result = HashMap::new();
    for keyword in keywords {
        *result.entry(keyword.clone()).or_default() += 1;
    }

    result
}

#[derive(Debug, PartialEq, Clone)]
pub struct CountDiff(i16);

#[derive(Debug, PartialEq, Clone)]
pub struct WarningCollectionDiff {
    /// Mapping of warning names to counts
    names: HashMap<String, i16>,

    /// Mapping of filenames to counts
    files: HashMap<PathBuf, i16>,

    /// Mapping of directory names to counts
    directories: HashMap<PathBuf, i16>,

    /// Mapping of keywords to counts
    keywords: HashMap<String, i16>,
}

impl WarningCollection {
    pub fn new<T: AsRef<str>>(
        content: &str,
        keyword_len: usize,
        ignored_keywords: &[T],
    ) -> WarningCollection {
        lazy_static! {
            static ref WARN_RE: Regex = Regex::new(
                r"(?x)
            (?P<file>.*):\d+:\d+:\s*                 # Filename
            (?P<text_before>\n\n\s+\d+\ \|.*\n.*\n)? # Possible source code (gfortran)
            [wW]arning:.*\[-W(?P<name>.*)\]          # Warning name
            (?P<text_after>\n\s+\d+\ \|.*)?          # Possible source code (gcc/clang)
            "
            )
            .unwrap();
        }

        let cwd = current_dir().unwrap_or(PathBuf::from(""));

        let result = WARN_RE
            .captures_iter(content)
            .map(|cap| Warning {
                name: String::from(&cap["name"]),
                file: {
                    let filename = PathBuf::from(&cap["file"]);
                    filename
                        .strip_prefix(&cwd)
                        .unwrap_or(&filename)
                        .to_path_buf()
                },
                keywords: match cap.name("text_after") {
                    Some(capture) => make_keywords(capture.as_str(), keyword_len, ignored_keywords),
                    _ => match cap.name("text_before") {
                        Some(capture) => {
                            make_keywords(capture.as_str(), keyword_len, ignored_keywords)
                        }
                        _ => Vec::new(),
                    },
                },
            })
            .collect::<Vec<_>>();

        let names = count_warning_types(&result);
        let files = count_warning_files(&result);
        let directories = count_warning_directories(&result);
        let keywords = count_warning_keywords(&result);

        WarningCollection {
            warnings: result,
            names,
            files,
            directories,
            keywords,
        }
    }

    pub fn diff(&self, other: &WarningCollection) -> WarningCollectionDiff {
        WarningCollectionDiff {
            names: diff_hashmaps(&self.names, &other.names),
            files: diff_hashmaps(&self.files, &other.files),
            directories: diff_hashmaps(&self.directories, &other.directories),
            keywords: diff_hashmaps(&self.keywords, &other.keywords),
        }
    }
}

fn diff_hashmaps<T>(lhs: &HashMap<T, i16>, rhs: &HashMap<T, i16>) -> HashMap<T, i16>
where
    T: Eq + Hash + Clone,
{
    let mut result = lhs.clone();

    for (name, count) in rhs.iter() {
        *result.entry(name.clone()).or_default() -= count;
    }
    result.retain(|_, &mut value| value != 0);
    result
}

impl fmt::Display for WarningCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let top_n = match f.precision() {
            Some(precision) => precision,
            _ => 10,
        };

        let names = make_warning_counts(&self.names, 0, false);
        let files = make_warning_counts(&self.files, top_n, true);
        let directories = make_warning_counts(&self.directories, top_n, true);
        let keywords = make_warning_counts(&self.keywords, top_n, true);
        write!(
            f,
            r#"Warnings:
{names}

Files:
{files}

Directories:
{directories}

Keywords:
{keywords}
"#
        )
    }
}

fn make_warning_counts<T: AsRef<Path>>(
    warnings: &HashMap<T, i16>,
    top_n: usize,
    use_total_items: bool,
) -> String
where
    T: Eq + Ord,
{
    if warnings.is_empty() {
        return String::new();
    }

    let mut count_vec: Vec<_> = warnings.iter().collect();
    count_vec.sort_by(|lhs, rhs| {
        if lhs.1 == rhs.1 {
            lhs.0.cmp(rhs.0)
        } else {
            lhs.1.cmp(rhs.1).reverse()
        }
    });

    let max_length = if top_n == 0 {
        count_vec.len()
    } else {
        std::cmp::min(count_vec.len(), top_n)
    };

    let min_width = warnings.values().sum::<i16>().ilog10() as usize + 1;

    let result = count_vec
        .iter()
        .take(max_length)
        .map(|line| format!(r"{1:0$}  {2}", min_width, line.1, line.0.as_ref().display()))
        .fold(String::default(), |acc, line| format!("{acc}{line}\n"));
    let extra = if count_vec.len() > top_n && top_n != 0 {
        format!(
            "{1:0$}  (+{2} more items)\n",
            min_width,
            " ",
            count_vec.len() - top_n
        )
    } else {
        "".to_string()
    };

    let total: i16 = if use_total_items {
        count_vec.len() as i16
    } else {
        warnings.values().sum()
    };
    let total_line = format!("{1:0$}  Total", min_width, total);

    result + &extra + &total_line
}

// Helper function from https://stackoverflow.com/a/45145246
#[cfg(test)]
macro_rules! vec_of_strings {
    // match a list of expressions separated by comma:
    ($($str:expr),*) => ({
        // create a Vec with this list of expressions,
        // calling String::from on each:
        vec![$(String::from($str),)*] as Vec<String>
    });
}

#[cfg(test)]
lazy_static! {
    static ref TEST_WARNINGS: WarningCollection = WarningCollection {
        warnings: Vec::from([
            Warning {
                file: std::path::PathBuf::from("/path/to/dir1/file1.c"),
                name: String::from("bad-thing"),
                keywords: vec_of_strings!["horrible", "zing", "zimb"],
            },
            Warning {
                file: std::path::PathBuf::from("/path/to/dir2/file1.c"),
                name: String::from("dont-like-this"),
                keywords: vec_of_strings!["zing", "zimb", "zang"],
            },
            Warning {
                file: std::path::PathBuf::from("/path/to/dir2/file2.c"),
                name: String::from("horrible-stuff"),
                keywords: vec_of_strings!["horrible", "stuff"],
            },
            Warning {
                file: std::path::PathBuf::from("/path/to/dir2/file2.c"),
                name: String::from("horrible-stuff"),
                keywords: vec_of_strings!["horrible", "stuff"],
            },
        ]),
        names: HashMap::from([
            ("bad-thing".to_string(), 1),
            ("dont-like-this".to_string(), 1),
            ("horrible-stuff".to_string(), 2),
        ]),
        files: HashMap::from([
            (PathBuf::from("/path/to/dir1/file1.c"), 1),
            (PathBuf::from("/path/to/dir2/file1.c"), 1),
            (PathBuf::from("/path/to/dir2/file2.c"), 2),
        ]),
        directories: HashMap::from([
            (PathBuf::from("/path/to/dir1"), 1),
            (PathBuf::from("/path/to/dir2"), 3)
        ]),
        keywords: HashMap::from([
            ("horrible".to_string(), 3),
            ("stuff".to_string(), 2),
            ("zang".to_string(), 1),
            ("zimb".to_string(), 2),
            ("zing".to_string(), 2),
        ])
    };
}

#[test]
fn find_a_warning() {
    let result = WarningCollection::new(
        "Some warnings
[  1%] Generating file1.c
[  2%] Generating file2.c
/path/to/dir1/file1.c: In function ‘func1’:
/path/to/dir1/file1.c:235:36: warning: doing some bad thing [-Wbad-thing]
  235 |     if (horrible) *foo = zing->zimb;
      |                                ^~~~~
/path/to/dir2/file1.c: In function ‘func2’:
/path/to/dir2/file1.c:340:27: warning: don't like this [-Wdont-like-this]
  340 |     zing->zimb &= (~foo.zang);
      |                     ^~
/path/to/dir2/file2.c: In function ‘func3’:
/path/to/dir2/file2.c:697:16: warning: just horrible stuff [-Whorrible-stuff]
  697 |     horrible = stuff;
      |                ^~~
/path/to/dir2/file2.c:715:18: warning: just horrible stuff [-Whorrible-stuff]
  715 |       horrible = stuff[i];
      |                  ^~~
",
        3,
        &["foo"],
    );

    assert_eq!(result, *TEST_WARNINGS);
    assert_eq!(result.names, TEST_WARNINGS.names);
    assert_eq!(result.files, TEST_WARNINGS.files);
    assert_eq!(result.directories, TEST_WARNINGS.directories);
    assert_eq!(result.keywords, TEST_WARNINGS.keywords);
}

#[test]
fn find_a_warning_fortran() {
    let result = WarningCollection::new(
        "Some warnings
[  1%] Generating file1.c
[  2%] Generating file2.c
/path/to/dir1/file1.c:235:36:

  235 |     if (horrible) *foo = zing->zimb;
      |                                1
Warning: doing some bad thing [-Wbad-thing]
/path/to/dir2/file1.c:340:27:

  340 |     zing->zimb &= (~foo.zang);
      |                     ^
Warning: don't like this [-Wdont-like-this]
/path/to/dir2/file2.c:697:16:

  697 |     horrible = stuff;
      |                ^~~
Warning: just horrible stuff [-Whorrible-stuff]
/path/to/dir2/file2.c:715:18:

  715 |       horrible = stuff[i];
      |                  ^~~
Warning: just horrible stuff [-Whorrible-stuff]
",
        3,
        &["foo"],
    );

    assert_eq!(result, *TEST_WARNINGS);
    assert_eq!(result.names, TEST_WARNINGS.names);
    assert_eq!(result.files, TEST_WARNINGS.files);
    assert_eq!(result.directories, TEST_WARNINGS.directories);
    assert_eq!(result.keywords, TEST_WARNINGS.keywords);
}

#[test]
fn format_hash_map_for_warnings() {
    let counts = HashMap::from([
        ("result1".to_string(), 3),
        ("result2".to_string(), 120),
        ("result3".to_string(), 1),
    ]);

    let result = make_warning_counts(&counts, 2, false);
    let expected = "120  result2\n  3  result1\n     (+1 more items)\n124  Total".to_string();
    assert_eq!(result, expected);
}

#[test]
fn format_hash_map_for_files() {
    let counts = HashMap::from([
        ("result1".to_string(), 3),
        ("result2".to_string(), 120),
        ("result3".to_string(), 1),
    ]);

    let result = make_warning_counts(&counts, 2, true);
    let expected = "120  result2\n  3  result1\n     (+1 more items)\n  3  Total".to_string();
    assert_eq!(result, expected);
}

#[test]
fn warning_diff() {
    let new_warnings = WarningCollection {
        warnings: Vec::from([
            Warning {
                file: std::path::PathBuf::from("/path/to/dir1/file1.c"),
                name: String::from("bad-thing"),
                keywords: vec_of_strings!["horrible", "zing", "zimb"],
            },
            Warning {
                file: std::path::PathBuf::from("/path/to/dir2/file1.c"),
                name: String::from("dont-like-this"),
                keywords: vec_of_strings!["zing", "zimb", "zang"],
            },
        ]),
        names: HashMap::from([
            ("bad-thing".to_string(), 1),
            ("dont-like-this".to_string(), 1),
        ]),
        files: HashMap::from([
            (PathBuf::from("/path/to/dir1/file1.c"), 1),
            (PathBuf::from("/path/to/dir2/file1.c"), 1),
        ]),
        directories: HashMap::from([
            (PathBuf::from("/path/to/dir1"), 1),
            (PathBuf::from("/path/to/dir2"), 1),
        ]),
        keywords: HashMap::from([
            ("horrible".to_string(), 1),
            ("zang".to_string(), 1),
            ("zimb".to_string(), 2),
            ("zing".to_string(), 2),
        ]),
    };
    let result = new_warnings.diff(&TEST_WARNINGS);

    let expected = WarningCollectionDiff {
        names: HashMap::from([("horrible-stuff".to_string(), -2)]),
        files: HashMap::from([(PathBuf::from("/path/to/dir2/file2.c"), -2)]),
        directories: HashMap::from([(PathBuf::from("/path/to/dir2"), -2)]),
        keywords: HashMap::from([("horrible".to_string(), -2), ("stuff".to_string(), -2)]),
    };

    assert_eq!(result, expected);
}
