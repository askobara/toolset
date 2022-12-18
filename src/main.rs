#[macro_use] extern crate lazy_static;
#[macro_use] extern crate prettytable;
extern crate skim;
extern crate xdg;

#[macro_use] extern crate log;
extern crate simplelog;

use simplelog::TermLogger;
use chrono_humanize::HumanTime;

use arboard::Clipboard;
use chrono::prelude::*;
use clap_complete::{generate, Generator, Shell};
use clap::{Parser, Command, CommandFactory, Subcommand};
use console::style;
use prettytable::format::{TableFormat, FormatBuilder, LinePosition, LineSeparator};
use prettytable::Table;
use serde::{Deserialize, Serialize};
use skim::prelude::*;
use std::io;
use std::cmp::Ordering;
use struct_field_names_as_array::FieldNamesAsArray;
use reqwest::header;
use std::fs::File;
use std::io::{Write, Read};

mod settings;
mod build;
mod deploy;
mod normalize;

use crate::normalize::*;
use crate::settings::*;

lazy_static! {
    pub static ref CONFIG: Settings = {
        Settings::new().unwrap()
    };

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
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {

    #[command()]
    RunBuild {
        #[arg(short, long)]
        branch_name: Option<String>,
        #[arg(short, long)]
        workdir: Option<String>,
    },

    #[command()]
    RunDeploy {
        #[arg(short, long)]
        build_id: Option<String>,
        #[arg(short, long)]
        env: Option<String>,
        #[arg(long)]
        workdir: Option<String>,
        #[arg(long)]
        build_type: Option<String>,
    },

    #[command()]
    ListBuilds {
        #[arg(long)]
        workdir: Option<String>,
        /// use "any" as a value to disable filter, current branch name is using by default.
        #[arg(long)]
        branch_name: Option<String>,
        /// use "any" as a value to disable filter, a build type associated with workdir is
        /// using by default. Values "build", "b", "deploy" and "d" also will work.
        #[arg(long)]
        build_type: Option<String>,
        /// use "any" as a value to disable filter, an user associated with current token is using by
        /// default.
        #[arg(long)]
        author: Option<String>,
        #[arg(short, long)]
        limit: Option<u8>,
    },

    #[command()]
    ListBuildTypes {
    },

    #[command()]
    Init {
    },
}

// enum BuildTypeType {
//     Regular = "regular",
//     Deployment = "deployment",
//     Composite = "composite",
// }

#[derive(Debug, Serialize, Deserialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
pub struct BuildType {
    id: String,
    name: String,
    project_name: String,
    project_id: String,
    href: String,
    web_url: String,
    r#type: Option<String>,
}

impl SkimItem for BuildType {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.id)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(format!("{:#?}", self))
    }
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
    build_type: BuildType,
    wait_reason: String,
    queued_date: String,
    triggered: Triggered,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    id: i32,
    build_type_id: String,
    number: Option<String>,
    status: Option<String>, // SUCCESS/FAILURE/UNKNOWN
    state: String, // queued/running/finished
    branch_name: Option<String>,
    href: String,
    web_url: String,
    finish_on_agent_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Builds {
    count: i32,
    href: String,
    next_href: Option<String>,
    prev_href: Option<String>,
    build: Vec<Build>,
}

#[derive(Debug, Serialize, Deserialize, FieldNamesAsArray)]
#[serde(rename_all = "camelCase")]
#[field_names_as_array(rename_all = "camelCase")]
struct BuildTypes {
    count: i32,
    href: String,
    next_href: Option<String>,
    prev_href: Option<String>,
    build_type: Vec<BuildType>,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn create_client() -> reqwest::Client {
    let mut headers = header::HeaderMap::new();

    // {host}/profile.html?item=accessTokens
    let token = format!("Bearer {}", CONFIG.teamcity.auth_token);
    // Consider marking security-sensitive headers with `set_sensitive`.
    let mut auth_value = header::HeaderValue::from_str(&token).unwrap();
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);

    headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("application/json"));

    reqwest::Client::builder()
        .default_headers(headers)
        .build().unwrap()
}

pub fn save_as_last_build(build: &BuildQueue) {
    let path = xdg::BaseDirectories::with_prefix("teamcity").ok()
        .and_then(|xdg_dir| xdg_dir.place_state_file(build.build_type_id.clone()).ok())
        .unwrap();

    let mut file = File::create(&path).unwrap();
    write!(&mut file, "{}", build.id).unwrap();
}

pub fn get_last_build(build_type: &str) -> Option<i32> {
    let path = xdg::BaseDirectories::with_prefix("teamcity").ok()
        .and_then(|xdg_dir| Some(xdg_dir.get_state_file(build_type)))
        .unwrap();

    File::open(&path).ok().and_then(|mut file| {
        let mut str = String::new();
        let _ = file.read_to_string(&mut str);
        str.parse().ok()
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    let cli = Cli::parse();

    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        eprintln!("Generating completion file for {:?}...", generator);
        print_completions(generator, &mut cmd);

        return Ok(());
    } else if let Some(command) = cli.command {
        let client = create_client();

        match command {
            Commands::RunBuild { branch_name, workdir } => {
                let build = crate::build::run_build(&client, workdir.as_deref(), branch_name.as_deref()).await?;

                let mut clipboard = Clipboard::new().unwrap();
                if clipboard.set_text(build.web_url).is_ok() {
                    // FIXME: x11 will clear the clipboard when program is exit
                    println!("{}", style("✔ copied!").green().italic());
                }
            },

            Commands::ListBuilds { workdir, branch_name, build_type, author, limit } => {
                let path = normalize_path(workdir.as_deref());
                let branch = normalize_branch_name(branch_name.as_deref(), &path);
                let btype = normalize_build_type(build_type.as_deref(), &path);

                let mut locator: Vec<String> = vec![
                    format!("defaultFilter:false"),
                    format!("personal:false"),
                    format!("count:{}", limit.unwrap_or(5))
                ];

                if branch != "any" {
                    locator.push(format!("branch:{}", branch));
                } else {
                    locator.push(format!("branch:default:any"));
                }

                if btype == "build" || btype == "b" {
                    locator.push(format!("buildType:(type:regular,name:Build)"));
                } else if btype == "deploy" || btype == "d" {
                    locator.push(format!("buildType:(type:deployment)"));
                } else if btype != "any" {
                    locator.push(format!("buildType:{}", btype));
                }

                if let Some(author) = author {
                    locator.push(format!("user:{}", author));
                }

                let url = format!(
                    "{host}/app/rest/builds?locator={locator}",
                    host = CONFIG.teamcity.host,
                    locator = locator.join(",")
                );

                info!("{}", &url);

                let response = client.get(url)
                    .send()
                    .await?
                    .json::<Builds>()
                    .await?
                ;

                let mut table = Table::new();
                table.set_format(*TABLE_FORMAT);

                table.set_titles(row!["", "date", "build type", "build id", "url (branch)"]);
                for b in response.build {
                    table.add_row(row![
                        match b.status.as_deref().unwrap_or("UNKNOWN") {
                            "SUCCESS" => format!("{}", style("✓").green().bold()),
                            "FAILURE" => format!("{}", style("✗").red().bold()),
                            "UNKNOWN" => format!("{}", style("?").bold()),
                            _ => format!("unexpected status")
                        },
                        format!(
                            "{} {}",
                            match &b.state[..] {
                                "queued" => "祥queued",
                                "running" => "痢running",
                                "finished" => "",
                                _ => "?"
                            },
                            b.finish_on_agent_date
                                .and_then(|str| DateTime::parse_from_str(&str, "%Y%m%dT%H%M%S%z").map(|dt| dt.with_timezone(&chrono::Local)).ok())
                                .and_then(|date| {
                                    let delta = chrono::Local::now() - chrono::Duration::hours(1);
                                    let value = match date.partial_cmp(&delta) {
                                        Some(Ordering::Greater) => HumanTime::from(date).to_string(),
                                        _ => date.format("%a, %d %b %R").to_string()
                                    };

                                    Some(value)
                                })
                                .unwrap_or(String::default()),
                        ),
                        b.build_type_id,
                        b.id,
                        // style(format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\", url = b.web_url, text = b.number)),
                        format!(
                            "{url}\n{branch}",
                            url = style(b.web_url).blue().underlined(),
                            branch = b.branch_name.as_deref().unwrap_or("master (default branch)"),
                        ),
                    ]);
                }

                table.printstd();
            },

            Commands::ListBuildTypes {} => {
                let fields = format!("{}", normalize_field_names(&BuildTypes::FIELD_NAMES_AS_ARRAY)).replace(
                    "buildType",
                    &format!("buildType({})", normalize_field_names(&BuildType::FIELD_NAMES_AS_ARRAY))
                );

                let url = format!(
                    "{host}/app/rest/buildTypes?fields={fields}",
                    host = CONFIG.teamcity.host,
                    fields = fields,
                );

                println!("{:?}", &url);
                let response = client.get(url)
                    .send()
                    .await?
                    .json::<BuildTypes>()
                    .await?
                ;

                let options = SkimOptionsBuilder::default()
                    .height(Some("50%"))
                    .multi(true)
                    .preview(Some(""))
                    .build()
                    .unwrap();

                let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

                response.build_type.into_iter().for_each(|bt| {
                    let _ = tx_item.send(Arc::new(bt));
                });
                drop(tx_item); // so that skim could know when to stop waiting for more items.

                let selected_items = Skim::run_with(&options, Some(rx_item))
                    .map(|out| out.selected_items)
                    .unwrap_or_else(Vec::new);

                for item in selected_items.iter() {
                    println!("{}", item.output());
                }


                // let mut table = Table::new();
                // table.set_format(*TABLE_FORMAT);
                //
                // table.set_titles(row!["id", "name", "type"]);
                // for bt in response.build_type {
                //     table.add_row(row![bt.id, bt.name, bt.r#type.unwrap_or("None".to_string())]);
                // }
                // table.printstd();
            },

            Commands::RunDeploy { build_id, env, workdir, build_type } => {
                let response = crate::deploy::run_deploy(&client, build_id.as_deref(), env.as_deref(), workdir.as_deref(), build_type.as_deref()).await?;

                println!("{}", response.web_url);
            },

            Commands::Init {} => {

            },
        }

    }

    Ok(())
}
