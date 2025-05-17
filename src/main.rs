use anyhow::Result;
use chrono::prelude::*;
use clap::Parser;
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Carl,
    Staging,
    Prod,
}

impl std::str::FromStr for Stage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "carl" => Ok(Stage::Carl),
            "staging" => Ok(Stage::Staging),
            "prod" => Ok(Stage::Prod),
            _ => Err(format!(
                "Invalid stage: '{}'. Must be one of 'carl', 'staging', or 'prod'.",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FpgaTarget {
    Zcu104,
    Zcu104Nightly,
    Vck190,
}

impl std::str::FromStr for FpgaTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zcu104-nightly" => Ok(Self::Zcu104Nightly),
            "zcu104" => Ok(Self::Zcu104),
            "vck190" => Ok(Self::Vck190),
            _ => Err(format!(
                "Invalid fpga: '{}'. Must be one of 'zcu104', 'zcu104-nightly' or 'vck190'.",
                s
            )),
        }
    }
}

/// New type for self-hosted runner labels
struct RunnerLabels(Vec<String>);

impl RunnerLabels {
    fn new(value: FpgaTarget, dry_run: bool) -> Self {
        let postfix = if dry_run { "-staging" } else { "" };
        let inner = match value {
            FpgaTarget::Zcu104 => vec!["caliptra-fpga".to_string()],
            FpgaTarget::Zcu104Nightly => {
                vec![
                    "caliptra-fpga".to_string(),
                    "caliptra-fpga-nightly".to_string(),
                ]
            }
            FpgaTarget::Vck190 => {
                vec![format!("vck190{}", postfix)]
            }
        };
        Self(inner)
    }
}

/// New type for the runner name
/// Each runner name uses 16 random hexadecimal's to de-duplicate, in case a runner crashes and
/// isn't cleaned up.
struct RunnerName(String);
impl RunnerName {
    /// Type is the FPGA board type.
    /// Identifier is a unique number to differentiate boards running in the CI.
    /// Location is the physical location of the runner, for example "kir".
    fn new(fpga_type: FpgaTarget, identifier: &str, location: &str) -> Self {
        let board_type = match fpga_type {
            FpgaTarget::Zcu104 | FpgaTarget::Zcu104Nightly => "caliptra-fpga",
            FpgaTarget::Vck190 => "vck190",
        };

        let now = Local::now();
        let current_date = now.date_naive().format("%Y-%m-%d").to_string();

        // Generate a random hexadecimal postfix
        let mut rng = rand::rng();
        let rand_postfix: String = (0..16)
            .map(|_| format!("{:X}", rng.random::<u8>() % 16))
            .collect();

        let runner_name =
            format!("{board_type}-{location}-{identifier}-{rand_postfix}-{current_date}",);
        Self(runner_name)
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(value_enum, short, long, value_name = "STAGE")]
    stage: Stage,
    #[clap(value_enum, short, long, value_name = "FPGA_TARGET")]
    fpga_target: FpgaTarget,
    #[clap(short = 'i', long, value_name = "FPGA_IDENTIFIER")]
    fpga_identifier: String,
    #[clap(short = 'l', long, value_name = "LOCATION")]
    location: String,
    #[clap(short, long, value_name = "KEY_PATH")]
    key_path: String,
    #[clap(short, long)]
    dry_run: bool,
}

struct CaliptraCiInfo {
    github_app_id: u64,
    github_installation_id: u64,
    github_org_name: String,
    key_path: String,
}

impl From<Args> for CaliptraCiInfo {
    fn from(value: Args) -> Self {
        match value.stage {
            Stage::Carl => {
                // Set environment variables for carl
                CaliptraCiInfo {
                    github_app_id: 1160975,
                    github_installation_id: 61798278,
                    github_org_name: "clundin25-testorg".to_string(),
                    key_path: value.key_path,
                }
            }
            Stage::Staging => {
                // TODO: Set environment variables for staging
                todo!("TODO: Set environment variables for staging");
            }
            Stage::Prod => CaliptraCiInfo {
                github_app_id: 379559,
                github_installation_id: 40993215,
                github_org_name: "chipsalliance".to_string(),
                key_path: value.key_path,
            },
        }
    }
}

struct OctocrabWrapper {
    github_org_name: String,
    octocrab: octocrab::Octocrab,
}

impl OctocrabWrapper {
    fn new(info: &CaliptraCiInfo) -> Result<Self> {
        let github_org_name = info.github_org_name.clone();
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(&std::fs::read(info.key_path.clone())?)?;

        let octocrab = octocrab::Octocrab::builder()
            .app(info.github_app_id.into(), key)
            .build()?;
        // We already know the installation ID so we can optimistically add it to the `Octocrab`
        // object.
        let octocrab = octocrab.installation(info.github_installation_id.into())?;
        Ok(Self {
            github_org_name,
            octocrab,
        })
    }

    async fn runner_jit_token(&self, name: RunnerName, labels: RunnerLabels) -> Result<String> {
        // For Caliptra we only use one runner group.
        let default_runner_group = 1;

        let token = self
            .octocrab
            .actions()
            .create_org_jit_runner_config(
                self.github_org_name.clone(),
                name.0,
                default_runner_group.into(),
                labels.0,
            )
            .send()
            .await?;

        Ok(token.encoded_jit_config)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    eprintln!("Running for stage: {:?}", args.stage);

    let name = RunnerName::new(args.fpga_target, &args.fpga_identifier, &args.location);
    let labels = RunnerLabels::new(args.fpga_target, args.dry_run);
    let github = OctocrabWrapper::new(&args.into())?;

    match github.runner_jit_token(name, labels).await {
        Ok(jit_config) => {
            println!("{jit_config}");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Failed to create runner config due to {:}", e);
            std::process::exit(1);
        }
    }
}
