use crate::{Result, error, output, run};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

const USAGE: &str = "usage: deploy [-h|--help] [-u|--user] [-s|--src SRC_BRANCH] [-d|--deploy DEPLOY_BRANCH] [--verbose] [--no-push]";

pub fn deploy(args: &[String]) -> Result<()> {
    let mut source = "master";
    let mut destination = "gh-pages";
    let mut verbose = false;
    let mut no_push = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            "-u" | "--user" => {}
            "-s" | "--src" => {
                source = args
                    .next()
                    .ok_or_else(|| error(format!("Option {arg} requires a branch.\n{USAGE}")))?
            }
            "-d" | "--deploy" => {
                destination = args
                    .next()
                    .ok_or_else(|| error(format!("Option {arg} requires a branch.\n{USAGE}")))?
            }
            "--verbose" => verbose = true,
            "--no-push" => no_push = true,
            _ => return Err(error(format!("Option {arg} is unknown.\n{USAGE}"))),
        }
    }
    if source == destination {
        return Err(error("Source and deploy branches must differ."));
    }
    for branch in [source, destination] {
        output(
            Command::new("git").args(["check-ref-format", "--branch", branch]),
            verbose,
        )?;
    }
    let root = output(
        Command::new("git").args(["rev-parse", "--show-toplevel"]),
        verbose,
    )?;
    if fs::canonicalize(root.trim_end())? != std::env::current_dir()? {
        return Err(error("Run deploy from the repository root."));
    }
    println!("Deploying...\nSource branch: {source}\nDeploy branch: {destination}");
    print!("Do you want to proceed? [y/N] ");
    io::stdout().flush()?;
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    let response = response.trim().to_ascii_lowercase();
    let mut remaining = response.as_str();
    while !remaining.is_empty() {
        if let Some(rest) = remaining
            .strip_prefix("yes")
            .or_else(|| remaining.strip_prefix('y'))
        {
            remaining = rest;
        } else {
            break;
        }
    }
    if response.is_empty() || !remaining.is_empty() {
        println!("Aborting.");
        return Err(error(""));
    }
    let dirty = output(
        Command::new("git").args(["diff-index", "--name-only", "HEAD", "--"]),
        verbose,
    )?;
    if !dirty.is_empty() {
        println!(
            "Changes to the following files are uncommitted:\n{dirty}Please commit the changes before proceeding.\nAborting."
        );
        return Err(error(""));
    }
    let untracked = output(
        Command::new("git").args(["ls-files", "--exclude-standard", "--others"]),
        verbose,
    )?;
    if !untracked.is_empty() {
        println!(
            "There are untracked files:\n{untracked}Please commit those files or stash them before proceeding.\nAborting."
        );
        return Err(error(""));
    }
    if branch_exists(source, verbose)? {
        run(Command::new("git").args(["checkout", source]), verbose)?;
    } else {
        run(
            Command::new("git").args(["checkout", "-b", source]),
            verbose,
        )?;
    }
    if branch_exists(destination, verbose)? {
        run(
            Command::new("git").args(["branch", "-D", "--", destination]),
            verbose,
        )?;
    }
    run(
        Command::new("git").args(["checkout", "-b", destination]),
        verbose,
    )?;
    run(
        Command::new("bundle")
            .args(["exec", "jekyll", "build"])
            .env("JEKYLL_ENV", "production"),
        verbose,
    )?;
    run(
        Command::new("purgecss")
            .args(["-c", "purgecss.config.js"])
            .env("JEKYLL_ENV", "production"),
        verbose,
    )?;
    let site = fs::symlink_metadata("_site")?;
    if !site.is_dir() {
        return Err(error("Jekyll did not produce an _site directory."));
    }
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        if ["_site", ".git", "CNAME", ".gitignore"]
            .iter()
            .any(|name| entry.file_name() == *name)
        {
            continue;
        }
        remove(&entry.path())?;
    }
    for entry in fs::read_dir("_site")? {
        let entry = entry?;
        let target = Path::new(".").join(entry.file_name());
        if [".git", ".gitignore"]
            .iter()
            .any(|name| entry.file_name() == *name)
        {
            continue;
        }
        fs::rename(entry.path(), target)?;
    }
    fs::remove_dir_all("_site")?;
    fs::write(".nojekyll", "")?;
    run(Command::new("git").args(["add", "-fA"]), verbose)?;
    let message = output(
        Command::new("git").args(["log", "-1", "--pretty=%B"]),
        verbose,
    )?;
    run(
        Command::new("git").args([
            "commit",
            "--allow-empty",
            "-m",
            &format!("{} [ci skip]", message.trim_end_matches('\n')),
        ]),
        verbose,
    )?;
    if !no_push {
        run(
            Command::new("git").args(["push", "-f", "-q", "origin", destination]),
            verbose,
        )?;
    }
    run(Command::new("git").args(["checkout", source]), verbose)?;
    println!("Deployed successfully!");
    Ok(())
}

fn branch_exists(branch: &str, verbose: bool) -> Result<bool> {
    let mut command = Command::new("git");
    command.args([
        "show-ref",
        "--verify",
        "--quiet",
        &format!("refs/heads/{branch}"),
    ]);
    if verbose {
        eprintln!("+ {command:?}");
    }
    let status = command.status()?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => {
            crate::check(&command, status)?;
            Ok(false)
        }
    }
}

fn remove(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
