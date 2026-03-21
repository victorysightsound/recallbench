use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let task = args.first().map(|s| s.as_str()).unwrap_or("help");

    match task {
        "build-ui" => build_ui(),
        "build-all" => {
            build_ui();
            build_rust();
        }
        "dev-ui" => dev_ui(),
        "help" | "--help" | "-h" => print_help(),
        other => {
            eprintln!("Unknown task: {other}");
            print_help();
            std::process::exit(1);
        }
    }
}

fn ui_dir() -> PathBuf {
    project_root().join("recallbench/src/web/ui")
}

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // up from xtask/
    dir
}

fn build_ui() {
    let dir = ui_dir();
    println!("Building web UI...");

    // npm install (skip if node_modules exists)
    if !dir.join("node_modules").exists() {
        println!("  Running npm install...");
        run_cmd("npm", &["install"], &dir);
    }

    // npm run build
    println!("  Running npm run build...");
    run_cmd("npm", &["run", "build"], &dir);

    println!("Web UI built: recallbench/src/web/ui/dist/index.html");
}

fn build_rust() {
    println!("Building Rust workspace...");
    run_cmd("cargo", &["build", "--workspace"], &project_root());
}

fn dev_ui() {
    let dir = ui_dir();
    if !dir.join("node_modules").exists() {
        run_cmd("npm", &["install"], &dir);
    }
    println!("Starting Vite dev server...");
    run_cmd("npm", &["run", "dev"], &dir);
}

fn run_cmd(program: &str, args: &[&str], dir: &std::path::Path) {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| panic!("Failed to run {program}: {e}"));

    if !status.success() {
        eprintln!("{program} failed with {status}");
        std::process::exit(1);
    }
}

fn print_help() {
    println!("RecallBench build tasks

USAGE:
    cargo xtask <TASK>

TASKS:
    build-ui     Build the web UI (npm install + vite build)
    build-all    Build web UI then Rust workspace
    dev-ui       Start Vite dev server for UI development
    help         Show this help
");
}
