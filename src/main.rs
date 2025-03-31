use chrono::prelude::*;
use rand::Rng;
use clap::Parser;

use std::env;
use std::process::Command;


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
            _ => Err(format!("Invalid stage: '{}'. Must be one of 'carl', 'staging', or 'prod'.", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FpgaTarget {
    Zcu104,
    Vck190,
}

impl std::str::FromStr for FpgaTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zcu104" => Ok(Self::Zcu104),
            "vck190" => Ok(Self::Vck190),
            _ => Err(format!("Invalid fpga: '{}'. Must be one of 'zcu104' or 'vck190'.", s)),
        }
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
}

fn main() {
    let args = Args::parse();
    let stage = args.stage;

    eprintln!("Running for stage: {:?}", stage);

    let mut github_app_id: Option<&'static str> = None;
    let mut github_installation_id: Option<&'static str> = None;

    match stage {
        Stage::Carl => {
            // Set environment variables for carl
            env::set_var("GCP_ZONE", "us-central-1");
            env::set_var("GCP_PROJECT", "carl-caliptra-github-ci");
            env::set_var("GITHUB_ORG", "clundin25-testorg");
            github_app_id = Some("1160975");
            github_installation_id = Some("61798278");
        }
        Stage::Staging => {
            // TODO: Set environment variables for staging
            eprintln!("TODO: Set environment variables for staging");
        }
        Stage::Prod => {
            // TODO: Set environment variables for prod
            env::set_var("GCP_ZONE", "us-central-1");
            env::set_var("GCP_PROJECT", "caliptra-github-ci");
            env::set_var("GITHUB_ORG", "chipsalliance");
            github_app_id = Some("379559");
            github_installation_id = Some("40993215");
        }
    }

    let fpga = args.fpga_target;
    let fpga_target = match fpga {
        FpgaTarget::Zcu104 => "caliptra-fpga",
        FpgaTarget::Vck190 => "vck190",
    };

    // Generate a random hexadecimal postfix
    let mut rng = rand::rng();
    let rand_postfix: String = (0..16)
        .map(|_| format!("{:X}", rng.gen::<u8>() % 16))
        .collect();

    let now = Local::now();
    let current_date = now.date_naive().format("%Y-%m-%d").to_string();


    let fpga_identifier = args.fpga_identifier;
    let rtool_path = "/usr/local/google/home/clundin/code/caliptra-sw/ci-tools/github-runner/cmd/rtool/rtool";
    let jitconfig_arg = "jitconfig";
    let final_arg = format!("{}-kir-{}-{}-{}", fpga_target, fpga_identifier,rand_postfix, current_date);


    let mut command = Command::new(rtool_path);
    command.arg(jitconfig_arg)
        .arg(fpga_target)
        .arg(github_app_id.unwrap())
        .arg(github_installation_id.unwrap())
        .arg(final_arg);

    // Execute the command and handle the output
    match command.status() {
        Ok(status) => {
            if status.success() {
                eprintln!("rtool command executed successfully.");
            } else {
                eprintln!("Error: rtool command failed with status {:?}", status);
            }
        }
        Err(e) => {
            eprintln!("Error executing rtool command: {}", e);
        }
    }

    // If you need the output of the command, you can use the following:
    //match command.output() {
    //    Ok(output) => {
    //        if output.status.success() {
    //            println!("rtool output:\n{}", String::from_utf8_lossy(&output.stdout));
    //            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    //        } else {
    //            eprintln!("Error: rtool command failed with status {:?}", output.status);
    //            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    //        }
    //    }
    //    Err(e) => {
    //        eprintln!("Error executing rtool command: {}", e);
    //    }
    //}
}
