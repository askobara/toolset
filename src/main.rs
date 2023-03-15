#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate derive_builder;
extern crate skim;
extern crate colored_json;

use anyhow::Result;
use arboard::Clipboard;
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use console::style;
use prettytable::format::{FormatBuilder, LinePosition, LineSeparator, TableFormat};
use prettytable::Table;
use std::io;

mod normalize;
mod settings;
mod teamcity;
mod youtrack;

use crate::settings::*;
use crate::teamcity::ArgBuildType;

lazy_static! {
    static ref TABLE_FORMAT: TableFormat = FormatBuilder::new()
        .column_separator(' ')
        .separator(LinePosition::Top, LineSeparator::new('─', ' ', ' ', ' '))
        .separator(LinePosition::Title, LineSeparator::new('─', ' ', ' ', ' '))
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
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    workdir: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
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
        #[arg(short, long, conflicts_with = "branch_name")]
        build_id: Option<String>,
        #[arg(long)]
        branch_name: Option<String>,
        #[arg(short, long)]
        env: Option<String>,
    },

    #[command()]
    ListBuilds {
        #[arg(short, long, conflicts_with_all = ["branch_name", "build_type", "master"])]
        any: bool,
        #[arg(short, long, conflicts_with = "branch_name")]
        master: bool,
        #[arg(long, conflicts_with = "author")]
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
    BranchName {
        issue_id: String
    },

    #[command()]
    Init {},
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
        eprintln!("Generating completion file for {generator:?}...");
        print_completions(generator, &mut cmd);

        return Ok(());
    } else if let Some(command) = cli.command {
        let config = Settings::new()?;
        let client = teamcity::client::Client::new(&config.teamcity, cli.workdir.as_deref())?;

        match command {
            Commands::RunBuild { branch_name } => {
                let build = client.run_build(None, branch_name.as_deref()).await?;

                println!("{}", style(&build.web_url).bold().blue());

                let mut clipboard = Clipboard::new()?;
                if clipboard.set_text(build.web_url).is_ok() {
                    // FIXME: x11 will clear the clipboard when program is exit
                    println!("{}", style("✔ copied!").green().italic());
                }
            }

            Commands::RunDeploy { build_id, env, branch_name } => {
                let response = client
                    .run_deploy(build_id.as_deref(), env.as_deref(), branch_name.as_deref())
                    .await?;

                println!("{}", response.web_url);
            }

            Commands::ListBuilds {
                any,
                my,
                master,
                mut branch_name,
                mut build_type,
                mut author,
                limit,
            } => {
                if any {
                    branch_name.replace("any".into());
                    build_type.replace("any".into());
                } else if master {
                    branch_name.replace("master".into());
                }

                if my {
                    author.replace("current".into());
                }

                let builds = client
                    .get_builds(
                        branch_name.as_deref(),
                        build_type.as_ref(),
                        author.as_deref(),
                        limit,
                    )
                    .await?;

                let mut table = Table::new();
                table.set_format(*TABLE_FORMAT);

                table.set_titles(row![
                    "",
                    "date",
                    "build type",
                    "build id",
                    "url (branch)",
                    "triggered by"
                ]);
                for build in &builds {
                    table.add_row(row![
                        match build.status().unwrap_or("UNKNOWN") {
                            "SUCCESS" => format!("{}", style("✓").green().bold()),
                            "FAILURE" => format!("{}", style("✗").red().bold()),
                            "UNKNOWN" => format!("{}", style("?").bold()),
                            _ => "unexpected status".to_string(),
                        },
                        format!(
                            "{} {}",
                            match build.state() {
                                "queued" => "祥queued",
                                "running" => "痢running",
                                "finished" => "",
                                _ => "?",
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
                        build.triggered_by()
                    ]);
                }

                table.printstd();
            }

            Commands::Init {} => {
                unimplemented!()
            }

            Commands::BranchName { issue_id } => {
                let yt_client = crate::youtrack::client::Client::new(&config.youtrack, None)?;

                let issue = yt_client.get_issue_by_id(&issue_id).await?;

                println!("{}:{}", issue.as_local_branch_name(), issue.as_remote_branch_name());
            }
        }
    }

    Ok(())
}
