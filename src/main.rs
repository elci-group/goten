use std::process::{Command, Stdio};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Deserialize)]
struct PartResponse {
    text: String,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: goten <go-command|update> [args...]");
        std::process::exit(1);
    }

    if args[0] == "update" {
        match update_host_go_permanent() {
            Ok(_) => println!("Host Go update process completed."),
            Err(e) => eprintln!("Failed to update host Go: {}", e),
        }
        return;
    }

    if let Err(e) = try_update_host_go_session() {
        eprintln!("Note: Could not set session GOTOOLCHAIN: {}", e);
    }

    println!("Running command: go {}", args.join(" "));
    
    let output = run_command_capture(&args);
    if output.status.success() {
        println!("Command succeeded.");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        eprintln!("{}", stderr);
        
        println!("Command failed. Analyzing requirements for isolated environment...");
        let versions = detect_required_versions();
        let version = versions.iter().max_by(|a, b| compare_versions(a, b)).cloned().unwrap_or_else(|| "latest".to_string());
        println!("Selected dynamic version floor: {}", version);
        
        let container_output = run_in_container_capture(&args, &version);
        if !container_output.status.success() {
            let container_stderr = String::from_utf8_lossy(&container_output.stderr).to_string();
            eprintln!("{}", container_stderr);
            println!("Isolated build failed. Initiating Gemini 2.5 Pro analysis...");
            analyze_and_fix_with_gemini(&container_stderr);
        } else {
            println!("Container run succeeded.");
        }
    }
}

fn analyze_and_fix_with_gemini(error_log: &str) {
    let api_key = match fs::read_to_string(format!("{}/.keys/gemini", env::var("HOME").unwrap_or_default())) {
        Ok(k) => k.trim().to_string(),
        Err(_) => {
            eprintln!("Gemini API key not found in ~/.keys/gemini");
            return;
        }
    };

    let prompt = format!(
        "The following Go build failed in an isolated container. Analyze the error and provide a fix.\n\
        Error log:\n{}\n\n\
        Identify the file, the missing symbol, and suggest a diff to fix it.",
        error_log
    );

    let client = reqwest::blocking::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro-exp:generateContent?key={}",
        api_key
    );

    let body = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part { text: prompt }],
        }],
    };

    match client.post(url).json(&body).send() {
        Ok(resp) => {
            if let Ok(gemini_resp) = resp.json::<GeminiResponse>() {
                if let Some(candidate) = gemini_resp.candidates.first() {
                    if let Some(part) = candidate.content.parts.first() {
                        println!("\n=== Gemini 2.5 Pro Analysis & Fix ===\n");
                        println!("{}", part.text);
                        println!("\n=============================\n");
                    }
                }
            } else {
                eprintln!("Failed to parse Gemini response.");
            }
        }
        Err(e) => eprintln!("Gemini API request failed: {}", e),
    }
}

fn update_host_go_permanent() -> std::io::Result<()> {
    println!("Purging outdated Go installations and upgrading to latest...");
    let purge_paths = vec!["/usr/local/go", "/usr/lib/go", "/usr/bin/go", "/usr/local/bin/go"];
    for path in &purge_paths {
        if Path::new(path).exists() {
             let _ = Command::new("sudo").args(&["rm", "-rf", path]).status();
        }
    }
    let tarball = "go1.25.5.linux-amd64.tar.gz";
    let url = format!("https://go.dev/dl/{}", tarball);
    println!("Downloading latest Go from {}...", url);
    let download_status = Command::new("curl").args(&["-L", &url, "-o", tarball]).status();
    if download_status.is_ok() && download_status.unwrap().success() {
        Command::new("sudo").args(&["tar", "-C", "/usr/local", "-xzf", tarball]).status()?;
        Command::new("sudo").args(&["ln", "-sf", "/usr/local/go/bin/go", "/usr/local/bin/go"]).status()?;
        Command::new("sudo").args(&["ln", "-sf", "/usr/local/go/bin/gofmt", "/usr/local/bin/gofmt"]).status()?;
        fs::remove_file(tarball).unwrap_or_default();
        println!("Go 1.25.5 installed successfully.");
    }
    Ok(())
}

fn try_update_host_go_session() -> std::io::Result<()> {
    let output = Command::new("go").arg("version").output()?;
    let version_str = String::from_utf8_lossy(&output.stdout);
    if !version_str.contains("go1.18") && !version_str.contains("go1.17") {
        env::set_var("GOTOOLCHAIN", "auto");
    }
    Ok(())
}

fn detect_required_versions() -> HashSet<String> {
    let mut versions = HashSet::new();
    let root = find_project_root();
    let re = Regex::new(r"go\s*(?:>=)?\s*(\d+\.\d+(?:\.\d+)?)").unwrap();
    for entry in ["go.mod", "go.sum"] {
        let path = root.join(entry);
        if let Ok(content) = fs::read_to_string(path) {
            for cap in re.captures_iter(&content) {
                versions.insert(cap[1].to_string());
            }
        }
    }
    if versions.is_empty() { versions.insert("latest".to_string()); }
    versions
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    if a == "latest" { return std::cmp::Ordering::Greater; }
    if b == "latest" { return std::cmp::Ordering::Less; }
    let parse = |v: &str| v.split('.').map(|s| s.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    let va = parse(a);
    let vb = parse(b);
    va.cmp(&vb)
}

fn run_command_capture(args: &[String]) -> std::process::Output {
    Command::new("go").args(args).output().expect("Failed to execute go")
}

fn find_project_root() -> PathBuf {
    let mut current = env::current_dir().unwrap();
    loop {
        let go_mod = current.join("go.mod");
        if go_mod.exists() {
            if let Ok(content) = fs::read_to_string(&go_mod) {
                if content.lines().any(|l| l.starts_with("module ")) { return current; }
            }
        }
        if let Some(parent) = current.parent() { current = parent.to_path_buf(); }
        else { return env::current_dir().unwrap(); }
    }
}

fn run_in_container_capture(args: &[String], version: &str) -> std::process::Output {
    let image = if version == "latest" || version < "1.25" { "golang:1.25".to_string() }
                else { format!("golang:{}", version) };
    let project_root = find_project_root();
    let current_dir = env::current_dir().unwrap();
    let relative_path = current_dir.strip_prefix(&project_root).unwrap_or(Path::new(""));

    let docker_args = vec![
        "run".to_string(), "--rm".to_string(), "--env-file".to_string(), "/dev/null".to_string(),
        "-v".to_string(), format!("{}:/src:ro", project_root.display()),
        "-v".to_string(), "goten_gopath:/go".to_string(), "-w".to_string(), "/app".to_string(),
        "-e".to_string(), "GOROOT=/usr/local/go".to_string(), "-e".to_string(), "GOPATH=/go".to_string(),
        "-e".to_string(), "GOCACHE=/tmp/go-cache".to_string(), "-e".to_string(), "GO111MODULE=on".to_string(),
        "-e".to_string(), "GOTOOLCHAIN=auto".to_string(), image.clone(), "sh".to_string(), "-c".to_string(),
        format!("cp -rn /src/. /app/ && cd /app/{} && go {}", relative_path.display(), args.join(" ")),
    ];

    println!("Executing isolated container run (Image: {})...", image);
    Command::new("docker").args(&docker_args).output().unwrap_or_else(|_| {
        Command::new("podman").args(&docker_args).output().expect("Failed to execute docker/podman")
    })
}
