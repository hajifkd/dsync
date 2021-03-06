extern crate clap;

use clap::Clap;
use dsync::commands;
use dsync::get_token;

#[derive(Clap)]
#[clap(version = "0.1", author = "Hajime Fukuda <hajifkd@gmail.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Clone(CloneCommand),
    Pull,
    Add(AddCommand),
    Init(InitCommand),
}

#[derive(Clap)]
struct CloneCommand {
    remote_path: String,
    local_path: Option<String>,
}

#[derive(Clap)]
struct AddCommand {
    local_path: String,
}

#[derive(Clap)]
struct InitCommand {
    local_path: String,
    remote_path: String,
}

async fn init(command: InitCommand) -> Result<(), Box<dyn std::error::Error>> {
    commands::init::init(&command.remote_path, &command.local_path).await
}

async fn add(command: AddCommand) -> Result<(), Box<dyn std::error::Error>> {
    commands::add::add(&command.local_path, std::env::current_dir()?).await
}

async fn pull(token: &str) -> Result<(), Box<dyn std::error::Error>> {
    commands::pull::pull(std::env::current_dir()?, token).await
}

async fn clone(command: CloneCommand, token: &str) -> Result<(), Box<dyn std::error::Error>> {
    let CloneCommand {
        remote_path,
        local_path,
    } = command;

    let local_path = if let Some(path) = local_path {
        path
    } else {
        let remote_paths: Vec<_> = remote_path.split('/').collect();
        if remote_paths.len() == 0 {
            return Err("local path must be specified".into());
        }
        if remote_paths[remote_paths.len() - 1] == "" {
            if remote_paths.len() == 1 {
                return Err("local path must be specified".into());
            }
            remote_paths[remote_paths.len() - 2].to_owned()
        } else {
            remote_paths[remote_paths.len() - 1].to_owned()
        }
    };

    commands::clone::clone(&remote_path, local_path, token).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::parse();
    let token = get_token().await?;

    match opts.subcmd {
        SubCommand::Clone(command) => clone(command, &token).await,
        SubCommand::Pull => pull(&token).await,
        SubCommand::Add(command) => add(command).await,
        SubCommand::Init(command) => init(command).await,
    }
}
