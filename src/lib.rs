use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashMap, env::current_dir, path::PathBuf};

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

pub fn find_warnings<T: AsRef<str>>(
    content: &str,
    keyword_len: usize,
    ignored_keywords: &[T],
) -> Result<Vec<Warning>> {
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
                    Some(capture) => make_keywords(capture.as_str(), keyword_len, ignored_keywords),
                    _ => Vec::new(),
                },
            },
        })
        .collect::<Vec<_>>();

    Ok(result)
}

fn count_warning_fn<F>(warnings: &[Warning], f: F) -> HashMap<String, i16>
where
    F: Fn(&Warning) -> String,
{
    let mut result = HashMap::new();

    for warning in warnings {
        let count = result.entry(f(warning)).or_insert(0);
        *count += 1;
    }

    return result;
}

pub fn count_warning_types(warnings: &[Warning]) -> HashMap<String, i16> {
    count_warning_fn(warnings, |warning| warning.name.clone())
}

pub fn count_warning_files(warnings: &[Warning]) -> HashMap<String, i16> {
    count_warning_fn(warnings, |warning| {
        warning.file.as_os_str().to_string_lossy().to_string()
    })
}

pub fn count_warning_directories(warnings: &[Warning]) -> HashMap<String, i16> {
    count_warning_fn(warnings, |warning| {
        warning
            .file
            .parent()
            .unwrap()
            .as_os_str()
            .to_string_lossy()
            .to_string()
    })
}

pub fn count_warning_keywords(warnings: &[Warning]) -> HashMap<String, i16> {
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

pub fn make_warning_counts(
    warnings: &HashMap<String, i16>,
    top_n: usize,
    use_total_items: bool,
) -> String {
    if warnings.is_empty() {
        return String::new();
    }

    let mut count_vec: Vec<_> = warnings.iter().collect();
    count_vec.sort_by(|lhs, rhs| lhs.1.cmp(rhs.1).reverse());

    let total: i16 = if use_total_items {
        count_vec.len() as i16
    } else {
        warnings.values().sum()
    };

    let min_width = warnings.values().sum::<i16>().ilog10() as usize + 1;

    let mut result = Vec::new();
    for line in count_vec {
        result.push(format!(r"{1:0$}  {2}", min_width, line.1, line.0));
    }

    let max_length = if top_n == 0 {
        result.len()
    } else {
        std::cmp::min(result.len(), top_n)
    };
    let results = result[..max_length].join("\n");
    let total_line = format!("\n{1:0$}  Total", min_width, total);

    let extra = if result.len() > top_n && top_n != 0 {
        format!(
            "\n{1:0$}  (+{2} more items)",
            min_width,
            " ",
            result.len() - top_n
        )
    } else {
        "".to_string()
    };

    results + &extra + &total_line
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
fn make_test_warnings() -> Vec<Warning> {
    lazy_static! {
        static ref RESULT: Vec<Warning> = Vec::from([
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
        ]);
    }

    RESULT.to_vec()
}

#[test]
fn find_a_warning() -> Result<()> {
    let result = find_warnings(
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
    )?;

    let expected = make_test_warnings();
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn find_a_warning_fortran() -> Result<()> {
    let result = find_warnings(
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
    )?;

    let expected = make_test_warnings();
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn count_warnings() -> Result<()> {
    let warning_counts = count_warning_types(&make_test_warnings());

    let expected_counts = HashMap::from([
        ("bad-thing".to_string(), 1),
        ("dont-like-this".to_string(), 1),
        ("horrible-stuff".to_string(), 2),
    ]);

    assert_eq!(warning_counts, expected_counts);
    Ok(())
}

#[test]
fn count_files() -> Result<()> {
    let warning_counts = count_warning_files(&make_test_warnings());

    let expected_counts = HashMap::from([
        ("/path/to/dir1/file1.c".to_string(), 1),
        ("/path/to/dir2/file1.c".to_string(), 1),
        ("/path/to/dir2/file2.c".to_string(), 2),
    ]);

    assert_eq!(warning_counts, expected_counts);
    Ok(())
}

#[test]
fn count_directories() -> Result<()> {
    let warning_counts = count_warning_directories(&make_test_warnings());

    let expected_counts = HashMap::from([
        ("/path/to/dir1".to_string(), 1),
        ("/path/to/dir2".to_string(), 3),
    ]);

    assert_eq!(warning_counts, expected_counts);
    Ok(())
}

#[test]
fn format_hash_map_for_warnings() -> Result<()> {
    let counts = HashMap::from([
        ("result1".to_string(), 3),
        ("result2".to_string(), 120),
        ("result3".to_string(), 1),
    ]);

    let result = make_warning_counts(&counts, 2, false);
    let expected = "120  result2\n  3  result1\n     (+1 more items)\n124  Total".to_string();
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn format_hash_map_for_files() -> Result<()> {
    let counts = HashMap::from([
        ("result1".to_string(), 3),
        ("result2".to_string(), 120),
        ("result3".to_string(), 1),
    ]);

    let result = make_warning_counts(&counts, 2, true);
    let expected = "120  result2\n  3  result1\n     (+1 more items)\n  3  Total".to_string();
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn count_keywords() -> Result<()> {
    let counts = count_warning_keywords(&make_test_warnings());
    let expected = HashMap::from([
        ("horrible".to_string(), 3),
        ("stuff".to_string(), 2),
        ("zang".to_string(), 1),
        ("zimb".to_string(), 2),
        ("zing".to_string(), 2),
    ]);
    assert_eq!(counts, expected);
    Ok(())
}
