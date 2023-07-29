/*
 File: main.rs
 Created Date: 29 Jul 2023
 Author: realbacon
 -----
 Last Modified: 30/07/2023 12:15:32
 Modified By: realbacon
 -----
 License  : MIT
 -----
*/

use std::fs::File;
use std::path::Path;
use std::thread;
use std::time::Duration;
extern crate term_size;
use crossterm::terminal::{Clear, ClearType};
use std::io::{self, Write};
use std::io::{BufRead, BufReader};
use std::process::exit;
use syntect::easy::HighlightLines;

use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;
fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let file_name = args.get(1).unwrap_or_else(|| {
        println!("File name required");
        exit(0)
        //hello
    });
    let path = Path::new(file_name); //world
    let (_, rows) = term_size::dimensions().unwrap();

    let file = File::open(path).unwrap_or_else(|_| {
        println!("File does not exist");
        exit(0) // long comment
    });
    let reader = BufReader::new(file);
    let mut lines = reader
        .lines()
        .take(rows)
        .map(|l| l.unwrap())
        .collect::<Vec<String>>();
    lines.last_mut().unwrap().push('\n');
    let cmt = find_comments(&lines, path.extension().unwrap().to_str().unwrap());
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();
    let syntax = syntax_set
        .find_syntax_by_extension(path.extension().unwrap().to_str().unwrap())
        .expect("Syntax not found");
    let theme = &theme_set.themes["base16-ocean.dark"];
    let mut theme = theme.clone();
    theme.settings.background = Some(syntect::highlighting::Color::BLACK);
    let mut h = HighlightLines::new(syntax, &theme);
    loop {
        clear_term();
        print_with_syntax(&lines, &mut h, &syntax_set);
        //println!("{:?}", lines);
        let (nls, hs) = apply_phx(lines, &cmt);
        lines = nls;
        lines = lines.iter_mut().map(|s| s.trim_end().to_string()).collect();
        if !hs {
            exit(0);
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn clear_term() {
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, Clear(ClearType::All)).unwrap();
}

fn print_with_syntax(lines: &Vec<String>, h: &mut HighlightLines<'_>, syntax_set: &SyntaxSet) {
    let mut res = String::new();
    for i in 0..lines.len() {
        let line = lines.get(i).unwrap();
        let mut line = line.clone();
        //if i < lines.len() - 1 {
        line.push('\n');
        //}
        let ranges = h.highlight_line(&line[..], &syntax_set).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        res.push_str(&escaped[..]);
    }
    let mut buffer = std::io::BufWriter::new(std::io::stdout());
    buffer.write(res.as_bytes()).unwrap();
}

fn find_comments(lines: &Vec<String>, ext: &str) -> Vec<(isize, isize, isize)> {
    let mut res = Vec::new();
    let comment_tag;
    if ext == "rs" {
        comment_tag = "//";
    } else if ext == "py" {
        comment_tag = "#";
    } else {
        comment_tag = "#";
    }
    for i in 0..lines.len() {
        for j in 0..lines[i].len() {
            if lines[i].len() as isize - j as isize >= 2 {
                if (&lines[i][j..lines[i].len()])
                    .to_string()
                    .starts_with(comment_tag)
                {
                    res.push((i as isize, j as isize, (lines[i].len() - j - 1) as isize));
                    break;
                }
            }
        }
    }

    res
}

fn apply_phx(lines: Vec<String>, cmt: &Vec<(isize, isize, isize)>) -> (Vec<String>, bool) {
    let mut lines = lines;
    let mut has_changed = false;
    for i in (0..lines.len() - 1).rev() {
        for j in 0..lines[i].len() {
            if not_a_comment(cmt, (i as isize, j as isize))
                && lines[i].get(j..j + 1).unwrap() != " "
            {
                // check si ya qq chose en dessous
                if [" ", "\n"].contains(&lines[i + 1].get(j..j + 1).unwrap_or(" "))
                    && not_a_comment(cmt, ((i + 1) as isize, j as isize))
                {
                    if j >= lines[i + 1].len() {
                        let to_add = j - lines[i + 1].len() + 1;
                        lines[i + 1].push_str(&" ".repeat(to_add));
                    }

                    let mut nl = lines[i + 1].clone().chars().collect::<Vec<char>>();
                    nl[j] = lines[i].chars().collect::<Vec<char>>()[j];
                    let nl: String = nl.iter().collect();
                    lines[i + 1] = nl;
                    let mut nl = lines[i].clone().chars().collect::<Vec<char>>();
                    nl[j] = ' ';
                    let nl: String = nl.iter().collect();
                    lines[i] = nl;
                    has_changed = true;
                } else {
                    match nearest(j, &lines[i + 1]) {
                        Direction::LEFT => {
                            if j > 0 && lines[i].get(j - 1..j).unwrap() == " " {
                                let mut nl = lines[i].clone().chars().collect::<Vec<char>>();
                                nl[j - 1] = nl[j];
                                nl[j] = ' ';
                                let nl: String = nl.iter().collect();
                                lines[i] = nl;
                                has_changed = true;
                            }
                        }
                        Direction::RIGHT => {
                            if lines[i].get(j + 1..j + 2).unwrap_or(" ") == " " {
                                let mut nl = lines[i].clone().chars().collect::<Vec<char>>();
                                if j + 1 == nl.len() {
                                    nl.push(' ')
                                }

                                nl[j + 1] = nl[j];
                                nl[j] = ' ';
                                let nl: String = nl.iter().collect();
                                lines[i] = nl;
                                has_changed = true;
                            }
                        }
                        Direction::None => (),
                    }
                }
            }
        }
    }
    (lines, has_changed)
}
enum Direction {
    LEFT,
    RIGHT,
    None,
}
fn nearest(idx: usize, nxt_line: &String) -> Direction {
    let (mut l, mut r) = (0, 0);
    // test right
    loop {
        r += 1;
        if nxt_line
            .trim_end()
            .get((idx + r)..(idx + r + 1))
            .unwrap_or(" ")
            == " "
        {
            break;
        }
    }
    // test left
    loop {
        l += 1;
        if nxt_line
            .trim_end()
            .get((idx - l)..(idx - l + 1))
            .unwrap_or(" ")
            == " "
        {
            break;
        }
    }
    if l > 3 && r > 3 {
        Direction::None
    } else if l > r {
        Direction::RIGHT
    } else {
        Direction::LEFT
    }
}

fn not_a_comment(cmt: &Vec<(isize, isize, isize)>, idx: (isize, isize)) -> bool {
    for cp in cmt.iter() {
        if idx.0 == cp.0 && idx.1 >= cp.1 && cp.1 + cp.2 >= idx.1 {
            return false;
        }
    }
    return true;
}
