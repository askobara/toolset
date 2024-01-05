#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate derive_builder;
extern crate colored_json;
extern crate skim;

use anyhow::{Context, Result};
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use clap_verbosity_flag::Verbosity;
use console::style;
use std::io;
use tracing_log::AsTrace;
use youtrack::issue::{BaseIssue, IssueShort};

mod core;
mod repo;
mod gitlab;
mod normalize;
mod settings;
mod teamcity;
mod youtrack;
mod table;

use crate::settings::*;
use crate::teamcity::ArgBuildType;
use crate::youtrack::issue::BranchNameWithIssueId;

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
    CreateBranch { issue_id: String },

    #[command()]
    PullRequests {},

    #[command()]
    CreatePullRequest {},

    #[command()]
    AddComment { text: String },

    #[command()]
    OpenIssue { id: Option<String> },

    #[command()]
    TimeTracking { id: Option<String> },

    #[command()]
    SubIssues { id: Option<String> },

    #[command()]
    CreateSubIssue { id: Option<String> },

    #[command()]
    Patch {},

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

        let repo = repo::Repo::new(cli.workdir.as_deref())?;
        let teamcity = teamcity::Client::new(&config.teamcity, &repo)?;

        match command {
            Commands::RunBuild { branch_name } => {
                let build = teamcity.run_build(None, branch_name.as_deref()).await?;

                println!("{}", style(&build.web_url).bold().blue());

                let _ = dump_to_clipboard(&build.web_url);
                println!("{}", style("✔ copied!").green().italic());
            }

            Commands::RunDeploy {
                build_id,
                env,
                branch_name,
            } => {
                let response = teamcity
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

                let builds = teamcity
                    .get_builds(
                        branch_name.as_deref(),
                        build_type.as_ref(),
                        author.as_deref(),
                        limit,
                    )
                    .await?;

                let mut table = table::Table::new(row![
                    "Date",
                    "Build Type",
                    "Build Id",
                    "Url (branch)",
                    "Triggered By"
                ]);

                for build in &builds {
                    let state = match (build.state(), build.status()) {
                        ("queued", _) => format!("{}{}", style("祥").bold(), style("queued").yellow()),
                        ("running", _) => format!("{}{}", style("痢").bold(), style("running").yellow()),
                        ("finished", Some("SUCCESS")) => format!("{} {}", style("").bold().green(), build.finished_at()),
                        ("finished", Some("FAILURE")) => format!("{} {}", style("").bold().red(), build.finished_at()),
                        (_, _) => "?".to_string(),
                    };

                    table.add_row(row![
                        state,
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
                let yt_client = crate::youtrack::Client::new(&config.youtrack)?;

                let issue: IssueShort = yt_client.get_issue_by_id(&issue_id).await?;

                println!("{}", issue.as_local_branch_name());
            }

            Commands::CreateBranch { issue_id } => {
                let yt_client = crate::youtrack::Client::new(&config.youtrack)?;

                let issue: IssueShort = yt_client.get_issue_by_id(&issue_id).await?;

                repo.fetch(None)?;
                repo.create_and_switch(&issue.as_local_branch_name())?;
            }

            Commands::PullRequests {  } => {
                let gitlab_client = crate::gitlab::Client::new(&config.gitlab)?;
                let branch_name = repo.normalize_branch_name(None)?;

                let prs = gitlab_client.get_pull_requests(&branch_name, crate::gitlab::pull_request::State::All).await?;

                let mut table = table::Table::new(row![
                    "Title",
                    "Url",
                    "",
                    "",
                    "",
                ]);

                for pr in &prs {
                    table.add_row(row![
                        pr.title,
                        style(&pr.web_url).blue().underlined(),
                        pr.has_conflicts,
                        pr.user_notes_count,
                        pr.blocking_discussions_resolved,
                    ]);
                }

                table.printstd();
            }

            Commands::CreatePullRequest {  } => {
                let gitlab_client = crate::gitlab::Client::new(&config.gitlab)?;
                let bn = repo.get_branch_name_meta(None)?;

                let remote_branch_name = bn.local_name.parse::<BranchNameWithIssueId>()
                    .map(|b| b.short_name())
                    .unwrap_or(bn.local_name.clone());

                dbg!(&bn);

                if repo.count_ahead_commits()? == 0 {
                    anyhow::bail!("Commit first!");
                }

                let basename = repo.get_name(None)?;
                let prjs = gitlab_client.find_project_by_name(&basename).await?;
                let prj = prjs.first().context("No prj found")?;
                println!("{:?}", prj);

                repo.set_upstream(&bn.local_name, &remote_branch_name, bn.oid)?;
                repo.push(&bn.refname, &remote_branch_name)?;

                let r = gitlab_client.create_pull_request(&prj, &bn, &remote_branch_name).await?;
                dbg!(&r);

                let _ = dump_to_clipboard(&r.web_url.as_str());
                println!("{}", style("✔ copied!").green().italic());
            }

            Commands::AddComment { text } => {
                let yt_client = crate::youtrack::Client::new(&config.youtrack)?;

                let bn = repo.get_branch_name_meta(None)?;

                let issue_id = bn.local_name.parse::<BranchNameWithIssueId>()
                    .map(|b| b.short_name())?;

                let response = yt_client.comment_create(&issue_id, &text).await?;
                dbg!(response);
            }

            Commands::OpenIssue { id } => {
                let bn = repo.get_branch_name_meta(None)?;

                let issue_id = bn.local_name.parse::<BranchNameWithIssueId>()
                    .map(|b| b.short_name()).ok();

                let issue = id.unwrap_or_else(|| issue_id.expect("No issue id was found"));
                open_browser(&format!("{}/issue/{}", config.youtrack.client.host, issue))?;
            }

            Commands::TimeTracking { id } => {
                let bn = repo.get_branch_name_meta(None)?;

                let issue_id = bn.local_name.parse::<BranchNameWithIssueId>()
                    .map(|b| b.short_name()).ok();

                let issue = id.unwrap_or_else(|| issue_id.expect("No issue id was found"));

                let timestamp = {
                    let now = chrono::offset::Local::now();

                    let date = inquire::DateSelect::new("Select a date:")
                        .with_default(now.date_naive())
                        .with_min_date(
                            now.checked_sub_days(chrono::Days::new(7))
                                .unwrap()
                                .date_naive(),
                        )
                        .with_max_date(now.date_naive())
                        .with_week_start(chrono::Weekday::Mon)
                        .prompt()?;

                    let t = chrono::naive::NaiveTime::parse_from_str("00:00:00", "%H:%M:%S")?;

                    format!("{}000", date.and_time(t).format("%s")).parse()?
                };

                let text = inquire::Text::new("Text:").prompt()?;
                let duration = inquire::Text::new("Duration:").prompt()?;
                // let u = yt_client.me().await?;

                let body = youtrack::time_tracking::TimeTracking {
                    text,
                    date: timestamp,
                    uses_markdown: true,
                    author: youtrack::time_tracking::Author {
                        id: "1-6".to_string(),
                    },
                    duration: youtrack::time_tracking::Duration {
                        presentation: duration,
                    }
                };

                let yt_client = crate::youtrack::Client::new(&config.youtrack)?;
                let response = yt_client.create_time_tracking(&issue, &body).await?;

                dbg!(response);
            }

            Commands::SubIssues { id } => {
                let yt_client = crate::youtrack::Client::new(&config.youtrack)?;
                let bn = repo.get_branch_name_meta(None)?;

                let issue_id = bn.local_name.parse::<BranchNameWithIssueId>()
                    .map(|b| b.short_name()).ok();

                let issue = id.unwrap_or_else(|| issue_id.expect("No issue id was found"));

                let response: Vec<IssueShort> = yt_client.get_sub_issues(&issue).await?;

                let mut table = table::Table::new(row![
                    "Id",
                    "Title",
                ]);

                for issue in &response {
                    table.add_row(row![
                        issue.id_readable(),
                        issue.summary(),
                    ]);
                }

                table.printstd();
            }

            Commands::CreateSubIssue { id } => {
                let yt_client = crate::youtrack::Client::new(&config.youtrack)?;
                let bn = repo.get_branch_name_meta(None)?;

                let issue_id = bn.local_name.parse::<BranchNameWithIssueId>()
                    .map(|b| b.short_name()).ok();

                let issue = id.unwrap_or_else(|| issue_id.expect("No issue id was found"));

                let response: Vec<IssueShort> = yt_client.get_sub_issues(&issue).await?;
                let c = response.iter().filter(|i| i.is_backend_sub_issue()).count();

                // let r = yt_client.search_issue_link("targetToSource: {subtask of}").await?;
                // dbg!(r);
                //
                // return Ok(());
                if c > 0 {
                    anyhow::bail!("Already has a [BE] task!");
                }

                let current: youtrack::issue::IssueLong = yt_client.get_issue_by_id(&issue).await?;

                let new_issue = yt_client.create_subtask(&current).await?;
                dbg!(&new_issue);
                let response = yt_client.link_issues(&current, &new_issue).await?;
                dbg!(response);

                let tags = yt_client.search_tags("[BE] Need approve").await?;
                if let Some(tag) = tags.first() {
                    let response = yt_client.add_tag_to_issue(&new_issue, &tag).await?;
                    dbg!(response);
                }
            }

            Commands::Patch {  } => {
                todo!()
                // validate git stage
                // git commit -m --amend
                // git push --force-with-lease
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

#[cfg(target_os = "linux")]
pub fn open_browser(url: &str) -> Result<()> {
    use std::process::Command;

    Command::new("firefox")
        .arg(url)
        .spawn()
        .map_err(|e| anyhow::format_err!("Failed to open browser: {}", e))?;

    Ok(())
}
