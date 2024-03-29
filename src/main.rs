//
//           Copyright r6eve 2019 -
//  Distributed under the Boost Software License, Version 1.0.
//     (See accompanying file LICENSE_1_0.txt or copy at
//           https://www.boost.org/LICENSE_1_0.txt)
//

#![allow(unused_doc_comments)]

use chrono::Local;
use lazy_static::lazy_static;
use regex::{bytes, Regex};
use std::env;
use std::fs::{self, File};
use std::io::prelude::*;
use std::io::{self, BufReader, Write};
use std::os::unix;
use std::path::Path;
use std::process::{self, Command};

lazy_static! {
    static ref TANLOG_DIR: String = {
        match env::var_os("TANLOG_DIR") {
            Some(v) => v.into_string().unwrap(),
            None => String::from("/tmp/tanlog"),
        }
    };

    static ref REMOVE: bytes::Regex = bytes::Regex::new(r"(?x)
        \a # Bell
        | \x1B \x5B .*? [\x40-\x7E] # CSI
        | \x1B \x5D .*? \x07 # Set terminal title
        | \x1B [\x40-\x5A\x5C\x5F] # 2 byte sequence
        | \x1B \x28 \x42 # ESC ( B
    ").unwrap();

    // Remove end-of-line CRs.
    static ref END_OF_LINE_CR: bytes::Regex = bytes::Regex::new(r"(?x)\s* \x0D* \x0A").unwrap();
    // Replace orphan CRs with LFs.
    static ref ORPHAN_CR: bytes::Regex = bytes::Regex::new(r"(?x)\s* \x0D").unwrap();
}

macro_rules! errorln {
    () => ({
        eprintln!("error");
        process::exit(1);
    });
    ($fmt:expr) => ({
        eprintln!($fmt);
        process::exit(1);
    });
    ($fmt:expr, $($arg:tt)*) => ({
        eprintln!($fmt, $($arg)*);
        process::exit(1);
    });
}

macro_rules! tmux_log_on {
    ($file:ident) => {{
        Command::new("tmux")
            .arg("pipe-pane")
            .arg(&format!("cat >>{}", $file))
            .spawn()
            .unwrap_or_else(|e| errorln!("Command failed: {}", e));
    }};
}

macro_rules! tmux_log_off {
    () => {{
        Command::new("tmux")
            .arg("pipe-pane")
            .arg(";")
            .spawn()
            .unwrap_or_else(|e| errorln!("Command failed: {}", e));
    }};
}

fn raw_to_san(raw: &str) -> String {
    raw.replace("/RAW/", "/")
}

fn ln_sf<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) {
    unix::fs::symlink(src, dst).unwrap_or(());
}

fn rm_f<P: AsRef<Path>>(path: P) {
    fs::remove_file(path).unwrap_or(());
}

fn create_prev_links(logfile: &str, dir: &str) -> io::Result<()> {
    let mut prev_link = format!("{}/PPPPPPPPPP", dir); // "P" * 10
    for _ in 0..9 {
        prev_link.pop();
        if Path::new(&prev_link).exists() {
            fs::rename(&prev_link, &format!("{}P", prev_link))?;
        }
    }
    ln_sf(logfile, &prev_link);
    Ok(())
}

fn setup_cmd_link(logfile: &str, cmd: &str) -> io::Result<()> {
    let re = Regex::new(r"^[()\s]*(\S+)").unwrap();
    let cap = re.captures(cmd).unwrap();
    let arg0 = cap[1].parse::<String>().unwrap();
    let arg0 = Path::new(&arg0).file_name().unwrap();
    let arg0 = arg0.to_str().unwrap();

    let pp = Path::new(logfile).parent().unwrap();
    let pp = pp.to_str().unwrap();

    let cmddirs = [
        format!("{}/RAW/{}", *TANLOG_DIR, arg0),
        format!("{}/{}", pp, arg0),
    ];
    for cmddir in &cmddirs {
        for &(cd, lf) in &[
            (cmddir, logfile),
            (&raw_to_san(cmddir), &raw_to_san(logfile)),
        ] {
            fs::create_dir_all(cd)?;
            let p = Path::new(lf).file_name().unwrap();
            let p = p.to_str().unwrap();
            ln_sf(lf, &format!("{}/{}", cd, p));
            create_prev_links(lf, cd)?;
        }
    }
    Ok(())
}

fn start_tanlog(cmd: &str) -> io::Result<()> {
    let now = Local::now();
    let logdir = format!("{}/RAW/{}", *TANLOG_DIR, now.format("%Y-%m-%d"));

    for &(path, ld) in &[("RAW/", &logdir), ("", &raw_to_san(&logdir))] {
        fs::create_dir_all(ld)?;
        let dot_today = format!("{}/{}.TODAY", *TANLOG_DIR, path);
        rm_f(&dot_today);
        ln_sf(ld, &dot_today);
        fs::rename(&dot_today, &format!("{}/{}TODAY", *TANLOG_DIR, path))?;
    }

    let mut logfile = String::new();
    let time = now.format("%H:%M:%S");
    for n in 0.. {
        let lf = format!("{}/{}-{}.log", logdir, time, n);
        if !Path::new(&lf).exists() {
            logfile = lf;
            break;
        }
    }

    let mut f = File::create(&logfile)?;
    f.write_all(format!("$ {}\n", cmd).as_bytes())?;

    tmux_log_on!(logfile);

    print!("{}", logfile);

    create_prev_links(&raw_to_san(&logfile), &format!("{}/TODAY", *TANLOG_DIR))?;

    setup_cmd_link(&logfile, cmd)?;

    Ok(())
}

fn sanitize_log(rawfile: &str) -> io::Result<()> {
    let sanfile = raw_to_san(rawfile);
    if Path::new(&sanfile).exists() {
        process::exit(0)
    }
    lazy_static::initialize(&REMOVE);
    lazy_static::initialize(&END_OF_LINE_CR);
    lazy_static::initialize(&ORPHAN_CR);
    let ifile = File::open(rawfile)?;
    let mut br = BufReader::new(ifile);
    let mut of = File::create(&sanfile)?;
    let mut line = String::new();
    while let Ok(n) = br.read_line(&mut line) {
        if n == 0 {
            break;
        }
        let l = REMOVE.replace_all(line.as_bytes(), &b""[..]).into_owned();
        let l = END_OF_LINE_CR.replace_all(&l, &b"\x0A"[..]).into_owned();
        let l = ORPHAN_CR.replace_all(&l, &b"\x0A"[..]).into_owned();
        of.write_all(&l)?;
        line.clear();
    }
    Ok(())
}

fn end_tanlog(fname: &str) -> io::Result<()> {
    tmux_log_off!();
    if !Path::new(fname).exists() {
        process::exit(0)
    }
    let metadata = fs::metadata(fname)?;
    if metadata.len() >= 100_000_000 {
        process::exit(0)
    }
    sanitize_log(fname)?;
    Ok(())
}

fn show_recent_logs() -> io::Result<()> {
    let mut log = format!("{}/TODAY/PPPPPPPPPP", *TANLOG_DIR); // "P" * 10
    for _ in 0..10 {
        if Path::new(&log).exists() {
            let ifile = File::open(&log)?;
            let mut br = BufReader::new(ifile);
            let mut line = String::new();
            while br.read_line(&mut line).unwrap() > 0 {
                print!("{}", line);
                line.clear();
            }
        }
        log.pop();
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        errorln!("No args.")
    }
    if env::var_os("TMUX").is_none() {
        process::exit(0)
    }
    match args[0].as_ref() {
        "start" => {
            if args.len() < 2 {
                process::exit(0)
            }
            start_tanlog(&args[1])
        }
        "end" => {
            if args.len() < 2 {
                process::exit(0)
            }
            end_tanlog(&args[1])
        }
        "recent" => show_recent_logs(),
        "version" | "--version" => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            process::exit(0)
        }
        s => {
            eprintln!("Unknown option: {}", s);
            process::exit(1)
        }
    }
}
