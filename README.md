# warnsum

A little helper tool for summarising compiler warnings from log
files. Works on warnings generated from GCC/Clang and gfortran,
probably not on other compilers

## Why?

If you're working on a legacy project that generates thousands of
warnings, you might want some idea of where to start fixing things.

Also, this was just an excuse to learn Rust. Use at your own risk :)

## Installation

Install with
[cargo](https://doc.rust-lang.org/cargo/commands/cargo-install.html)
after cloning locally:

```bash
$ cargo install --path .
```

(or install directly from GitHub)

## Usage

Generate your compiler warnings and dump them to a file somehow, for
example:

```bash
$ cmake --build build |& tee make.log
```

then run `warnsum` on that file:

```bash
$ warnsum make.log
```

## Example

Give this set of warnings:

```
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
```

`warnsum` produces:

```
Warnings:
2  horrible-stuff
1  bad-thing
1  dont-like-this
4  Total

Files:
2  /path/to/dir2/file2.c
1  /path/to/dir1/file1.c
1  /path/to/dir2/file1.c
3  Total

Directories:
3  /path/to/dir2
1  /path/to/dir1
2  Total

Keywords:
3  horrible
2  stuff
2  Total
```

Note that the total for the `Warnings` category is the total number of
warnings, while for the other categories the total is the distinct
number of items in each category.
