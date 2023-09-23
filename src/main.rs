#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate derive_builder;
extern crate colored_json;
extern crate skim;

use anyhow::{Context, Result};
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use console::style;
use prettytable::format::{FormatBuilder, LinePosition, LineSeparator, TableFormat};
use prettytable::Table;
use clap_verbosity_flag::Verbosity;
use tracing_log::AsTrace;
use std::io;

mod core;
mod gitlab;
mod normalize;
mod settings;
mod teamcity;
mod youtrack;

use crate::settings::*;
use crate::teamcity::ArgBuildType;
use crate::youtrack::issue::BranchNameWithIssueId;

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
    #[command(flatten)]
    verbose: Verbosity,
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
    BranchName { issue_id: String },

    #[command()]
    PullRequests {},

    #[command()]
    CreatePullRequest {},

    #[command()]
    Init {},
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .init();

    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        eprintln!("Generating completion file for {generator:?}...");
        print_completions(generator, &mut cmd);

        return Ok(());
    } else if let Some(command) = cli.command {
        let config = Settings::new()?;

        let repo = normalize::find_a_repo(cli.workdir.as_deref())?;
        let client = teamcity::client::Client::new(&config.teamcity, &repo)?;

        match command {
            Commands::RunBuild { branch_name } => {
                let build = client.run_build(None, branch_name.as_deref()).await?;

                println!("{}", style(&build.web_url).bold().blue());

                let _ = dump_to_clipboard(&build.web_url);
                println!("{}", style("✔ copied!").green().italic());
            }

            Commands::RunDeploy {
                build_id,
                env,
                branch_name,
            } => {
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
                let yt_client = crate::youtrack::client::Client::new(&config.youtrack)?;

                let issue = yt_client.get_issue_by_id(&issue_id).await?;

                println!("{}", issue.as_local_branch_name());
            }

            Commands::PullRequests {  } => {
                let gitlab_client = crate::gitlab::Client::new(&config.gitlab)?;
                let branch_name = normalize::normalize_branch_name(None, &repo)?;

                let prs = gitlab_client.get_pull_requests(&branch_name, crate::gitlab::pull_request::State::All).await?;

                for pr in &prs {
                    let msg = format!("⚫ {}\n  {}", pr.title, pr.web_url);
                    println!("{msg}");
                }
            }

            Commands::CreatePullRequest {  } => {
                let gitlab_client = crate::gitlab::Client::new(&config.gitlab)?;
                let bn = normalize::get_branch_name_meta(None, &repo)?;

                let remote_branch_name = bn.local_name.parse::<BranchNameWithIssueId>()
                    .map(|b| b.short_name())
                    .unwrap_or(bn.local_name.clone());

                dbg!(&bn);

                let commited = {
                    let r = repo.lock().unwrap();
                    let mut revwalk = r.revwalk()?;
                    revwalk.push_range("origin/master..HEAD")?;

                    revwalk.count() > 0
                };

                if !commited {
                    anyhow::bail!("Commit first!");
                }

                let basename = normalize::get_repo_name(&repo, None)?;
                let prjs = gitlab_client.find_project_by_name(&basename).await?;
                let prj = prjs.first().context("No prj found")?;
                println!("{:?}", prj);

                {
                    let r = repo.lock().unwrap();
                    let mut b = r.find_branch(&bn.local_name.clone(), git2::BranchType::Local)?;


                    r.reference(
                        format!("refs/remotes/origin/{remote_branch_name}").as_str(),
                        bn.oid,
                        true,
                        ""
                    )?;
                    b.set_upstream(Some(format!("origin/{remote_branch_name}").as_str()))?;
                    let mut options = normalize::get_push_options();

                    let name = format!("{}:refs/heads/{}", bn.refname, remote_branch_name);

                    dbg!(&name);

                    r.find_remote("origin")?.push(
                        &[name.as_str()],
                        Some(&mut options)
                    )?;
                };

                let r = gitlab_client.create_pull_request(&prj, &bn).await?;
                dbg!(&r);

                let _ = dump_to_clipboard(&r.web_url.as_str());
                println!("{}", style("✔ copied!").green().italic());
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn dump_to_clipboard(value: &str) -> Result<()> {
    use std::{process::{Command, Stdio}, io::{Error, ErrorKind}};

    Command::new("echo")
        .arg(value)
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|echo| echo.stdout.ok_or(Error::new(ErrorKind::Other, "No stdout")))
        .and_then(|stdout|
            Command::new("xclip")
                .stdin(stdout)
                .args(["-selection", "c"])
                .status()
        )
        .map_err(|e| anyhow::format_err!("Failed to copy value to clipboard: {}", e))?;

    Ok(())
}
