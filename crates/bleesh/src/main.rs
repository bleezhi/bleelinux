//! bleesh – minimal shell for bleelinux. Phase 1: builtins + exec + pipes + redirect.
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};

fn main() -> anyhow::Result<()> {
    let mut rl = rustyline::DefaultEditor::new()?;
    loop {
        let cwd = env::current_dir().unwrap_or_else(|_| Path::new("/").to_path_buf());
        let prompt = format!("{} # ", cwd.display());
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() { continue; }
                let _ = rl.add_history_entry(line);
                if let Err(e) = run_line(line) {
                    eprintln!("bleesh: {}", e);
                }
            }
            Err(rustyline::error::ReadlineError::Eof) => break,
            Err(rustyline::error::ReadlineError::Interrupted) => continue,
            Err(e) => {
                eprintln!("bleesh: {}", e);
                break;
            }
        }
    }
    Ok(())
}

fn run_line(line: &str) -> anyhow::Result<()> {
    let stages: Vec<&str> = line.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if stages.is_empty() { return Ok(()); }
    if stages.len() == 1 {
        return run_simple(stages[0]);
    }
    // pipes
    let mut children: Vec<std::process::Child> = Vec::new();
    let mut prev_stdout: Option<std::process::ChildStdout> = None;
    for (i, stage) in stages.iter().enumerate() {
        let last = i + 1 == stages.len();
        let (prog, args, redir_in, redir_out) = parse_simple(stage)?;
        if prog.is_empty() { continue; }
        if is_builtin(&prog) {
            anyhow::bail!("builtin '{}' cannot be used in a pipe yet", prog);
        }
        let mut cmd = Command::new(&prog);
        cmd.args(&args);
        if let Some(out) = prev_stdout.take() {
            cmd.stdin(Stdio::from(out));
        } else if i == 0 {
            if let Some(f) = redir_in {
                cmd.stdin(Stdio::from(std::fs::File::open(f)?));
            }
        }
        if !last {
            cmd.stdout(Stdio::piped());
        } else if let Some(f) = redir_out {
            cmd.stdout(Stdio::from(std::fs::File::create(f)?));
        }
        let mut child = cmd.spawn()?;
        prev_stdout = if !last { child.stdout.take() } else { None };
        children.push(child);
    }
    for mut c in children {
        let _ = c.wait()?;
    }
    Ok(())
}

fn run_simple(line: &str) -> anyhow::Result<()> {
    let (prog, args, redir_in, redir_out) = parse_simple(line)?;
    if prog.is_empty() { return Ok(()); }
    match prog.as_str() {
        "cd" => {
            let dir = args.first().map(|s| s.as_str()).unwrap_or("/");
            env::set_current_dir(dir)?;
            return Ok(());
        }
        "exit" => std::process::exit(args.first().and_then(|s| s.parse().ok()).unwrap_or(0)),
        "pwd" => {
            println!("{}", env::current_dir()?.display());
            return Ok(());
        }
        "export" => {
            for a in args {
                if let Some((k, v)) = a.split_once('=') {
                    env::set_var(k, v);
                }
            }
            return Ok(());
        }
        _ => {}
    }
    let mut cmd = Command::new(&prog);
    cmd.args(&args);
    if let Some(f) = redir_in {
        cmd.stdin(Stdio::from(std::fs::File::open(f)?));
    }
    if let Some(f) = redir_out {
        cmd.stdout(Stdio::from(std::fs::File::create(f)?));
    }
    let mut child = cmd.spawn()?;
    let _ = child.wait()?;
    Ok(())
}

fn parse_simple(line: &str) -> anyhow::Result<(String, Vec<String>, Option<String>, Option<String>)> {
    let mut toks: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut sq = false;
    let mut dq = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' if !dq => sq = !sq,
            '"' if !sq => dq = !dq,
            '\\' if !sq => {
                if let Some(n) = chars.next() { cur.push(n); }
            }
            ' ' | '\t' if !sq && !dq => {
                if !cur.is_empty() { toks.push(cur.clone()); cur.clear(); }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() { toks.push(cur); }
    let mut args: Vec<String> = Vec::new();
    let mut redir_in = None;
    let mut redir_out = None;
    let mut prog = String::new();
    let mut i = 0;
    while i < toks.len() {
        match toks[i].as_str() {
            "<" if i + 1 < toks.len() => { redir_in = Some(toks[i+1].clone()); i += 2; }
            ">" if i + 1 < toks.len() => { redir_out = Some(toks[i+1].clone()); i += 2; }
            t => {
                if prog.is_empty() { prog = t.to_string(); }
                else { args.push(t.to_string()); }
                i += 1;
            }
        }
    }
    Ok((prog, args, redir_in, redir_out))
}

fn is_builtin(p: &str) -> bool {
    matches!(p, "cd" | "exit" | "pwd" | "export")
}
