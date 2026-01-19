use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DevcontainerConfig {
    name: String,

    image: Option<ImageConfig>,
    build: Option<BuildConfig>,
    compose: Option<ComposeConfig>,

    run: Option<RunConfig>,
    ports: Option<PortsConfig>,
    #[serde(default)]
    volumes: HashMap<String, VolumeMount>,
}

#[derive(Debug, Deserialize)]
struct ImageConfig {
    name: String,
}

#[derive(Debug, Deserialize)]
struct BuildConfig {
    name: String,
    dockerfile: String,
    context: String,
    target: Option<String>,
    #[serde(default)]
    args: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ComposeConfig {
    files: Vec<String>,
    service: Option<String>,
    workspace_folder: Option<String>,
    shutdown_action: Option<String>,
    override_command: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RunConfig {
    workdir: String,
    user: Option<String>,
    #[serde(default)]
    run_args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PortsConfig {
    #[serde(default)]
    app: Vec<u16>,
    #[serde(default)]
    forward: Vec<u16>,
}

#[derive(Debug, Deserialize)]
struct VolumeMount {
    host: String,
    container: String,
    mode: Option<String>,
}


fn load_config() -> Result<DevcontainerConfig, String> {
    let path = Path::new(".devcontainer").join("devcontainer.toml");

    let contents = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    toml::from_str::<DevcontainerConfig>(&contents)
        .map_err(|e| format!("Failed to read {} as TOML: {e}", path.display()))
}

fn print_help() {
    println!(
"Usage: devcontainer [COMMAND] [OPTIONS]

Commands:
    build        Build or prebuild devcontainer image(s)
    up           Start devcontainer(s)
    exec         Execute a command in a devcontainer
    stop         Stop devcontainer(s)
    down         Stop and remove devcontainer(s)
    read         Print devcontainer config

Options:
    -h, --help   Print this help message"
    );
}

fn handle_build(_args: &[String], cfg: &DevcontainerConfig) {
    let kind_count =
        cfg.image.is_some() as u8 + cfg.build.is_some() as u8 + cfg.compose.is_some() as u8;

    if kind_count == 0 {
        eprintln!("devcontainer build: no container kind specified");
        eprintln!(" Expected one of [image], [build], or [compose] in devcontainer.toml");
        process::exit(1);
    } else if kind_count > 1 {
        eprintln!("devcontainer build: multiple container kinds specified");
        eprintln!(" Only one of [image], [build], or [compose] can be set in devcontainer.toml");
        process::exit(1);
    }

    if let Some(image_cfg) = &cfg.image {
        run_image_build(image_cfg);
    } else if let Some(build_cfg) = &cfg.build {
        run_docker_build(build_cfg);
    } else if let Some(compose_cfg) = &cfg.compose {
        run_compose_build(compose_cfg);
    }
}

fn run_image_build(cfg: &ImageConfig) {
    let mut cmd = process::Command::new("docker");
    cmd.arg("pull").arg(&cfg.name);

    println!("devcontainer build: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker pull: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker pull failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn run_docker_build(cfg: &BuildConfig) {
    let mut cmd = process::Command::new("docker");
    cmd.arg("build")
        .arg("-f")
        .arg(&cfg.dockerfile);

    cmd.arg("-t").arg(&cfg.name);

    if let Some(target) = &cfg.target {
        cmd.arg("--target").arg(target);
    }

    for (k, v) in &cfg.args {
        cmd.arg("--build-arg").arg(format!("{k}={v}"));
    }

    cmd.arg(&cfg.context);

    println!("devcontainer build: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker build: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker build failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn run_compose_build(cfg: &ComposeConfig) {
    let mut cmd = process::Command::new("docker");
    cmd.arg("compose");

    for file in &cfg.files {
        cmd.arg("-f").arg(file);
    }

    cmd.arg("build");

    println!("devcontainer build: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker compose build: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker compose build failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn handle_up(_args: &[String], cfg: &DevcontainerConfig) {
    let kind_count =
        cfg.image.is_some() as u8 + cfg.build.is_some() as u8 + cfg.compose.is_some() as u8;

    if kind_count == 0 {
        eprintln!("devcontainer build: no container kind specified");
        eprintln!(" Expected one of [image], [build], or [compose] in devcontainer.toml");
        process::exit(1);
    } else if kind_count > 1 {
        eprintln!("devcontainer build: multiple container kinds specified");
        eprintln!(" Only one of [image], [build], or [compose] can be set in devcontainer.toml");
        process::exit(1);
    }

    if let Some(compose_cfg) = &cfg.compose {
        run_compose_up(compose_cfg);
    } else {
        run_container_up(cfg);
    }
}

fn run_container_up(cfg: &DevcontainerConfig) {
    let image_name = if let Some(image_cfg) = &cfg.image {
        image_cfg.name.clone()
    } else if let Some(build_cfg) = &cfg.build {
        build_cfg.name.clone()
    } else {
        unreachable!()
    };

    let mut cmd = process::Command::new("docker");
    cmd.arg("run")
        .arg("-d");

    let container_name = &cfg.name;
    cmd.arg("--name").arg(container_name);

    if let Some(run_cfg) = &cfg.run {
        if let Some(user) = &run_cfg.user {
            cmd.arg("--user").arg(user);
        }
        cmd.arg("--workdir").arg(&run_cfg.workdir);

        for extra in &run_cfg.run_args {
            cmd.arg(extra);
        }
    }

    for (_name, mount) in &cfg.volumes {
        let mode = mount.mode.as_deref().unwrap_or("rw");
        let spec = format!("{}:{}:{}", mount.host, mount.container, mode);
        cmd.arg("-v").arg(spec);
    }

    if let Some(ports_cfg) = &cfg.ports {
        for port in &ports_cfg.app {
            let mapping = format!("{p}:{p}", p = port);
            cmd.arg("-p").arg(mapping);
        }

        for port in &ports_cfg.forward {
            let mapping = format!("{p}:{p}", p = port);
            cmd.arg("-p").arg(mapping);
        }
    }

    cmd.arg(&image_name);

    println!("devcontainer up: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker run: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker run failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn run_compose_up(cfg: &ComposeConfig) {
    let mut cmd = process::Command::new("docker");
    cmd.arg("compose");

    for file in &cfg.files {
        cmd.arg("-f").arg(file);
    }

    cmd.arg("up")
        .arg("-d");

    if let Some(service) = &cfg.service {
        cmd.arg(service);
    }

    println!("devcontainer up: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker compose up: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker compose up failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn handle_exec(args: &[String], cfg: &DevcontainerConfig) {
    if let Some(compose_cfg) = &cfg.compose {
        run_compose_exec(args, compose_cfg);
    } else {
        run_container_exec(args, cfg);
    }
}

fn run_container_exec(args: &[String], cfg: &DevcontainerConfig) {
    let container_name = &cfg.name;

    let mut cmd = process::Command::new("docker");
    cmd.arg("exec")
        .arg("-it")
        .arg(container_name);

    if args.is_empty() {
        cmd.arg("sh");
    } else {
        for arg in args {
            cmd.arg(arg);
        }
    }

    println!("devcontainer exec: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker exec: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker exec failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn run_compose_exec(args: &[String], cfg: &ComposeConfig) {
    let mut cmd = process::Command::new("docker");
    cmd.arg("compose");

    for file in &cfg.files {
        cmd.arg("-f").arg(file);
    }

    cmd.arg("exec").arg("-it");

    let service = match &cfg.service {
        Some(s) => s,
        None => {
            eprintln!("devcontainer exec: [compose].service is required to know which compose service to exec into");
            std::process::exit(1);
        }
    };
    cmd.arg(service);

    if args.is_empty() {
        cmd.arg("sh");
    } else {
        for arg in args {
            cmd.arg(arg);
        }
    }

    println!("devcontainer exec: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker exec: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker exec failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn handle_stop(_args: &[String], cfg: &DevcontainerConfig) {
    if let Some(compose_cfg) = &cfg.compose {
        run_compose_stop(compose_cfg);
    } else {
        run_container_stop(cfg);
    }
}

fn run_container_stop(cfg: &DevcontainerConfig) {
    let container_name = &cfg.name;

    let mut cmd = process::Command::new("docker");
    cmd.arg("stop").arg(container_name);

    println!("devcontainer stop: running {:?}", cmd);


    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker stop: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker stop failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn run_compose_stop(cfg: &ComposeConfig) {
    let mut cmd = process::Command::new("docker");
    cmd.arg("compose");
    for file in &cfg.files {
        cmd.arg("-f").arg(file);
    }
    cmd.arg("stop");

    println!("devcontainer stop: running {:?}", cmd);


    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker compose stop: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker compose stop failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn handle_down(_args: &[String], cfg: &DevcontainerConfig) {
    if let Some(compose_cfg) = &cfg.compose {
        run_compose_down(compose_cfg);
    } else {
        run_container_down(cfg);
    }
}

fn run_container_down(cfg: &DevcontainerConfig) {
    let container_name = &cfg.name;

    let mut cmd = process::Command::new("docker");
    cmd.arg("rm").arg("-f").arg(container_name);

    println!("devcontainer down: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker rm: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker rm failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }
}

fn run_compose_down(cfg: &ComposeConfig) {
    let mut cmd = process::Command::new("docker");
    cmd.arg("compose");
    for file in &cfg.files {
        cmd.arg("-f").arg(file);
    }
    cmd.arg("down");

    println!("devcontainer down: running {:?}", cmd);

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to execute docker compose down: {e}");
            process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("docker compose down failed with status: {status}");
        process::exit(status.code().unwrap_or(1));
    }

}


fn handle_read(_args: &[String], cfg: &DevcontainerConfig) {
    println!("{:#?}", cfg);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let cfg = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("devcontainer: {e}");
            process::exit(1);
        }
    };

    if args.len() <= 1 {
        print_help();
        return;
    }

    let first = &args[1];

    match first.as_str() {
        "-h" | "--help" | "help" => {
            print_help();
        }

        "build" => {
            let build_args = &args[2..];
            handle_build(build_args, &cfg);
        }

        "up" => {
            let up_args = &args[2..];
            handle_up(up_args, &cfg);
        }

        "exec" => {
            let exec_args = &args[2..];
            handle_exec(exec_args, &cfg);
        }

        "stop" => {
            let stop_args = &args[2..];
            handle_stop(stop_args, &cfg);
        }

        "down" => {
            let down_args = &args[2..];
            handle_down(down_args, &cfg);
        }

        "read" => {
            let read_args = &args[2..];
            handle_read(read_args, &cfg)
        }

        other => {
            eprintln!("devcontainer: unknown command or option: {other}");
            eprintln!();
            eprintln!("Run 'devcontainer --help' for more information");

            // Exit with error
            process::exit(1);
        }
    }
}
