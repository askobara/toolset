#[macro_use] extern crate lazy_static;
#[macro_use] extern crate prettytable;
#[macro_use] extern crate derive_builder;
extern crate skim;

use anyhow::Result;
use arboard::Clipboard;
use clap_complete::{generate, Generator, Shell};
use clap::{Parser, Command, CommandFactory, Subcommand};
use console::style;
use prettytable::format::{TableFormat, FormatBuilder, LinePosition, LineSeparator};
use prettytable::Table;
use serde::{Deserialize, Serialize};
use std::io;

mod normalize;
mod settings;
mod client;
mod build;
mod deploy;
mod build_type;
mod build_locator;

use crate::settings::*;

lazy_static! {
    static ref TABLE_FORMAT: TableFormat = FormatBuilder::new()
        .column_separator(' ')
        .separator(LinePosition::Top,    LineSeparator::new('─', ' ', ' ', ' '))
        .separator(LinePosition::Title,  LineSeparator::new('─', ' ', ' ', ' '))
        .separator(LinePosition::Intern, LineSeparator::new('┈', ' ', ' ', ' '))
        .separator(LinePosition::Bottom, LineSeparator::new('─', ' ', ' ', ' '))
        .padding(1, 1)
        .build();
}

#[derive(Debug, Parser)]
#[command(name = "teamcity", author, version, about, long_about = None)] // Read from `Cargo.toml`
struct Cli {
    // If provided, outputs the completion file for given shell
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,
    #[arg(long)]
    workdir: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Clone)]
pub enum ArgBuildType {
    Build,
    Deploy,
    Any,
    Custom(String),
}

impl std::convert::From<&str> for ArgBuildType {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "build"|"b" => ArgBuildType::Build,
            "deploy"|"d" => ArgBuildType::Deploy,
            "any" => ArgBuildType::Any,
            custom @ _ => ArgBuildType::Custom(custom.to_string()),
        }
    }
}

impl std::convert::From<ArgBuildType> for String {
    fn from(v: ArgBuildType) -> Self {
        match v {
            ArgBuildType::Build => "build".into(),
            ArgBuildType::Deploy => "deploy".into(),
            ArgBuildType::Any => "any".into(),
            ArgBuildType::Custom(custom) => custom,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command()]
    RunBuild {
        #[arg(short, long)]
        branch_name: Option<String>,
    },

    #[command()]
    RunDeploy {
        #[arg(short, long)]
        build_id: Option<String>,
        #[arg(short, long)]
        env: Option<String>,
    },

    #[command()]
    ListBuilds {
        #[arg(short, long, conflicts_with_all=["branch_name", "build_type", "master"])]
        any: bool,
        #[arg(short, long, conflicts_with="branch_name")]
        master: bool,
        #[arg(long, conflicts_with="author")]
        my: bool,
        /// use "any" as a value to disable filter, current branch name is using by default.
        #[arg(long)]
        branch_name: Option<String>,
        /// use "any" as a value to disable filter, a build type associated with workdir is
        /// using by default. Values "build", "b", "deploy" and "d" also will work.
        #[arg(long)]
        build_type: Option<ArgBuildType>,
        /// use "any" as a value to disable filter, an user associated with current token is using by
        /// default.
        #[arg(long)]
        author: Option<String>,
        #[arg(short, long)]
        limit: Option<u8>,
    },

    #[command()]
    Init {
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    username: String,
    name: String,
    id: u32,
    href: String
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Triggered {
    r#type: String,
    date: String,
    user: User,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildQueue {
    id: i32,
    build_type_id: String,
    state: String,
    branch_name: Option<String>,
    href: String,
    web_url: String,
    // build_type: BuildType,
    wait_reason: String,
    queued_date: String,
    triggered: Triggered,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        eprintln!("Generating completion file for {:?}...", generator);
        print_completions(generator, &mut cmd);

        return Ok(());
    } else if let Some(command) = cli.command {

        let config = Settings::new()?;
        let client = client::Client::new(&config.teamcity, cli.workdir.as_deref())?;

        match command {
            Commands::RunBuild { branch_name } => {
                let build = client.run_build(None, branch_name.as_deref()).await?;

                println!("{}", style(&build.web_url).bold().blue());

                let mut clipboard = Clipboard::new()?;
                if clipboard.set_text(build.web_url).is_ok() {
                    // FIXME: x11 will clear the clipboard when program is exit
                    println!("{}", style("✔ copied!").green().italic());
                }
            },

            Commands::RunDeploy { build_id, env } => {
                let response = client.run_deploy(build_id.as_deref(), env.as_deref()).await?;

                println!("{}", response.web_url);
            },

            Commands::ListBuilds { any, my, master, mut branch_name, mut build_type, mut author, limit } => {
                if any {
                    branch_name.replace("any".into());
                    build_type.replace("any".into());
                } else if master {
                    branch_name.replace("master".into());
                }

                if my {
                    author.replace("current".into());
                }

                let builds = client.get_builds(branch_name.as_deref(), build_type.as_ref(), author.as_deref(), limit).await?;

                let mut table = Table::new();
                table.set_format(*TABLE_FORMAT);

                table.set_titles(row!["", "date", "build type", "build id", "url (branch)"]);
                for build in &builds {
                    table.add_row(row![
                        match build.status().unwrap_or("UNKNOWN") {
                            "SUCCESS" => format!("{}", style("✓").green().bold()),
                            "FAILURE" => format!("{}", style("✗").red().bold()),
                            "UNKNOWN" => format!("{}", style("?").bold()),
                            _ => "unexpected status".to_string()
                        },
                        format!(
                            "{} {}",
                            match build.state() {
                                "queued" => "祥queued",
                                "running" => "痢running",
                                "finished" => "",
                                _ => "?"
                            },
                            build.finished_at()
                        ),
                        build.build_type_id(),
                        build.id,
                        // style(format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\", url = build.web_url, text = build.number)),
                        format!(
                            "{url}\n{branch}",
                            url = style(build.web_url()).blue().underlined(),
                            branch = build.branch_name().unwrap_or("master (default branch)"),
                        ),
                    ]);
                }

                table.printstd();
            },

            Commands::Init {} => {
                unimplemented!()
            },
        }

    }

    Ok(())
}
