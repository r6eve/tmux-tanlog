#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate time;

use regex::{Regex, bytes};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::io::prelude::*;
use std::os::unix;
use std::path::Path;
use std::process::{self, Command};

lazy_static! {
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

const TANLOG_DIR: &'static str = "/tmp/tanlog";

macro_rules! errorln {
    () => ({
        writeln!(io::stderr(), "error").unwrap();
        process::exit(1);
    });
    ($fmt:expr) => ({
        writeln!(io::stderr(), $fmt).unwrap();
        process::exit(1);
    });
    ($fmt:expr, $($arg:tt)*) => ({
        writeln!(io::stderr(), $fmt, $($arg)*).unwrap();
        process::exit(1);
    });
}

macro_rules! tmux_log_on {
    ($file:ident) => ({
        Command::new("tmux").arg("pipe-pane").arg(&format!("cat >>{}", $file)).spawn()
            .unwrap_or_else(|e| errorln!("Command failed: {}", e.description()));
    });
}

macro_rules! tmux_log_off {
    () => ({
        Command::new("tmux").arg("pipe-pane").arg(";").spawn()
            .unwrap_or_else(|e| errorln!("Command failed: {}", e.description()));
    });
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

fn create_prev_links(logfile: &str, dir: &str) {
    let mut prev_link = format!("{}/PPPPPPPPPP", dir); // "P" * 10
    for _ in 0..9 {
        prev_link.pop().unwrap();
        if Path::new(&prev_link).exists() {
            fs::rename(&prev_link, &format!("{}P", prev_link)).unwrap();
        }
    }
    ln_sf(logfile, &prev_link);
}

fn setup_cmd_link(logfile: &str, cmd: &str) {
    let cap = Regex::new(r"^[()\s]*(\S+).*$").unwrap().captures(cmd).unwrap();
    let arg0 = cap[1].parse::<String>().unwrap();
    let arg0 = Path::new(&arg0).file_name().unwrap().to_str().unwrap();
    for cmddir in &[
        format!("{}/RAW/{}", TANLOG_DIR, arg0),
        format!("{}/{}", Path::new(logfile).parent().unwrap().to_str().unwrap(), arg0)
    ] {
        for t in &[(cmddir, logfile), (&raw_to_san(cmddir), &raw_to_san(logfile))] {
            fs::create_dir_all(t.0).unwrap();
            ln_sf(t.1, &format!("{}/{}", t.0, Path::new(t.1).file_name().unwrap().to_str().unwrap()));
            create_prev_links(t.1, t.0);
        }
    }
}

fn start_tanlog(cmd: &str) {
    let now = time::now();
    let logdir = format!("{}/RAW/{}", TANLOG_DIR, now.strftime("%Y-%m-%d").unwrap());

    for t in &[("RAW/", &logdir), ("", &raw_to_san(&logdir))] {
        fs::create_dir_all(t.1).unwrap();
        let dot_today = format!("{}/{}.TODAY", TANLOG_DIR, t.0);
        rm_f(&dot_today);
        ln_sf(t.1, &dot_today);
        fs::rename(&dot_today, &format!("{}/{}TODAY", TANLOG_DIR, t.0)).unwrap();
    }

    let logfile;
    let time = now.strftime("%H:%M:%S").unwrap();
    let mut n = 0;
    loop {
        let lf = format!("{}/{}-{}.log", logdir, time, n);
        if Path::new(&lf).exists() { n += 1; continue }
        logfile = lf.clone();
        break;
    }

    {
        let mut f = File::create(&logfile).unwrap();
        let s = format!("$ {}\n", cmd);
        f.write_all(s.as_bytes()).unwrap();
    }

    tmux_log_on!(logfile);

    write!(io::stdout(), "{}", logfile).unwrap();

    create_prev_links(&raw_to_san(&logfile), &format!("{}/TODAY", TANLOG_DIR));
    setup_cmd_link(&logfile, cmd);
}

fn sanitize_log(rawfile: &str) {
    let sanfile = raw_to_san(rawfile);
    if Path::new(&sanfile).exists() { process::exit(0) }
    lazy_static::initialize(&REMOVE);
    lazy_static::initialize(&END_OF_LINE_CR);
    lazy_static::initialize(&ORPHAN_CR);
    let ifile = File::open(rawfile).unwrap();
    let mut br = BufReader::new(ifile);
    let mut of = File::create(&sanfile).unwrap();
    let mut line = String::new();
    while br.read_line(&mut line).unwrap() > 0 {
        let l = REMOVE.replace_all(line.as_bytes(), &b""[..]).into_owned();
        let l = END_OF_LINE_CR.replace_all(&l, &b"\x0A"[..]).into_owned();
        let l = ORPHAN_CR.replace_all(&l, &b"\x0A"[..]).into_owned();
        of.write_all(&l).unwrap();
        line.clear();
    }
}

fn end_tanlog(fname: &str) {
    tmux_log_off!();
    if !Path::new(fname).exists() { process::exit(0) }
    let metadata = fs::metadata(fname).unwrap();
    if metadata.len() >= 100_000_000 { process::exit(0) }
    sanitize_log(fname);
}

fn show_recent_logs() {
    let mut log = format!("{}/TODAY/PPPPPPPPPP", TANLOG_DIR); // "P" * 10
    for _ in 0..10 {
        if Path::new(&log).exists() {
            let ifile = File::open(&log).unwrap();
            let mut br = BufReader::new(ifile);
            let mut line = String::new();
            while br.read_line(&mut line).unwrap() > 0 {
                write!(io::stdout(), "{}", line).unwrap();
                line.clear();
            }
        }
        log.pop().unwrap();
    }
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();
    if args.is_empty() { errorln!("No args.") }
    if env::var_os("TMUX").is_none() { process::exit(0) }
    match args[0].as_ref() {
        "start" => {
            if args.len() < 2 { process::exit(0) }
            start_tanlog(&args[1]);
        }
        "end" => {
            if args.len() < 2 { process::exit(0) }
            end_tanlog(&args[1]);
        }
        "recent" => show_recent_logs(),
        _ => errorln!("Unknown tanlog command: {}", &args[0]),
    }
}
