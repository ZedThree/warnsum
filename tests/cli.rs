use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;


#[test]
fn file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("warnsum")?;

    cmd.arg("test/file/doesnt/exist");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("could not read file"));

    Ok(())
}

#[test]
fn find_content_in_file() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("sample.txt")?;
    file.write_str("Some warnings
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
")?;

    let mut cmd = Command::cargo_bin("warnsum")?;
    cmd.arg(file.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("2  horrible-stuff"));

    Ok(())
}
