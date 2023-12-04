use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct Warning {
    name: String,
    file: std::path::PathBuf,
}

pub fn find_warnings(content: &str) -> Result<Vec<Warning>> {
    lazy_static! {
        static ref WARN_RE: Regex =
            Regex::new(r"(?P<file>.*):\d+:\d+: warning:.*\[-W(?P<name>.*)\]").unwrap();
    }

    let result = WARN_RE
        .captures_iter(content)
        .map(|cap| Warning {
            name: String::from(&cap["name"]),
            file: std::path::PathBuf::from(&cap["file"]),
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

pub fn make_warning_counts(warnings: &HashMap<String, i16>, top_n: usize, use_total_items: bool) -> String {
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

#[test]
fn find_a_warning() -> Result<()> {
    let result = find_warnings(
        "Some warnings
[  1%] Generating file1.c
[  2%] Generating file2.c
/path/to/file1.c: In function ‘func1’:
/path/to/file1.c:235:36: warning: doing some bad thing [-Wbad-thing]
  235 |     if (bad_thing) *foo = zing->bat.pazz.zimb;
      |                                    ^~~~~
/path/to/file1.c: In function ‘func2’:
/path/to/file1.c:340:27: warning: don't like this [-Wdont-like-this]
  340 |     zing->zapp.zoom &= (~zaff);
      |                           ^~
/path/to/file2.c: In function ‘func3’:
/path/to/file2.c:697:16: warning: just horrible stuff [-Whorrible-stuff]
  697 |     horrible = stuff;
      |                ^~~
/path/to/file2.c:715:18: warning: just horrible stuff [-Whorrible-stuff]
  715 |       horrible = stuff[i];
      |                  ^~~
",
    )?;

    let expected = [
        Warning {
            file: std::path::PathBuf::from("/path/to/file1.c"),
            name: String::from("bad-thing"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file1.c"),
            name: String::from("dont-like-this"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file2.c"),
            name: String::from("horrible-stuff"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file2.c"),
            name: String::from("horrible-stuff"),
        },
    ];
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn count_warnings() -> Result<()> {
    let warnings = [
        Warning {
            file: std::path::PathBuf::from("/path/to/file1.c"),
            name: String::from("bad-thing"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file1.c"),
            name: String::from("dont-like-this"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file2.c"),
            name: String::from("horrible-stuff"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file2.c"),
            name: String::from("horrible-stuff"),
        },
    ];
    let warning_counts = count_warning_types(&warnings);

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
    let warnings = [
        Warning {
            file: std::path::PathBuf::from("/path/to/file1.c"),
            name: String::from("bad-thing"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file1.c"),
            name: String::from("dont-like-this"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file2.c"),
            name: String::from("horrible-stuff"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/file2.c"),
            name: String::from("horrible-stuff"),
        },
    ];
    let warning_counts = count_warning_files(&warnings);

    let expected_counts = HashMap::from([
        ("/path/to/file1.c".to_string(), 2),
        ("/path/to/file2.c".to_string(), 2),
    ]);

    assert_eq!(warning_counts, expected_counts);
    Ok(())
}

#[test]
fn count_directories() -> Result<()> {
    let warnings = [
        Warning {
            file: std::path::PathBuf::from("/path/to/dir1/file1.c"),
            name: String::from("bad-thing"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/dir2/file1.c"),
            name: String::from("dont-like-this"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/dir2/file2.c"),
            name: String::from("horrible-stuff"),
        },
        Warning {
            file: std::path::PathBuf::from("/path/to/dir2/file2.c"),
            name: String::from("horrible-stuff"),
        },
    ];
    let warning_counts = count_warning_directories(&warnings);

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
