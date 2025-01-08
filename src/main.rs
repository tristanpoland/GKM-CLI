// src/main.rs
use clap::Command;
use anyhow::Result;
mod ui;
mod commands;
mod types;
mod constants;

use ui::GenesisKitUI;

#[tokio::main]
async fn main() -> Result<()> {
    let ui = GenesisKitUI::new();
    ui.display_welcome()?;

    let cli = Command::new("gk")
        .about("Genesis Kit Management Tool")
        .subcommand(Command::new("repipe").about("Update Concourse pipelines"))
        .subcommand(Command::new("template").about("Manage kit template versions"))
        .subcommand(Command::new("ci").about("Manage CI configuration"))
        .get_matches();

    match cli.subcommand() {
        Some(("repipe", _)) => ui.repipe_interactive().await?,
        Some(("template", _)) => ui.manage_template_version().await?,
        Some(("ci", _)) => ui.manage_ci().await?,
        _ => {
            println!("Please specify a command. Use --help for usage information.");
        }
    }

    Ok(())
}