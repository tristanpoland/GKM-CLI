use clap::Command;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select, MultiSelect};
use console::{style, Term};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use spinners::{Spinner, Spinners};
use std::{thread, time::Duration, fs};
use tabled::{Table, Tabled};
use anyhow::Result;
use semver::Version;

const LOGO: &str = r#"
 ██████╗ ██╗  ██╗███╗   ███╗
██╔════╝ ██║ ██╔╝████╗ ████║
██║  ███╗█████╔╝ ██╔████╔██║
██║   ██║██╔═██╗ ██║╚██╔╝██║
╚██████╔╝██║  ██╗██║ ╚═╝ ██║
 ╚═════╝ ╚═╝  ╚═╝╚═╝     ╚═╝
    Concept-0.0.1-alpha
"#;

#[derive(Debug, Tabled)]
struct KitStatus {
    #[tabled(rename = "Kit Name")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Template Version")]
    template_version: String,
    #[tabled(rename = "CI Status")]
    ci_status: String,
}

fn heading(text: &str) -> String {
    style(text).magenta().bold().to_string()
}

fn param(text: &str) -> String {
    style(text).yellow().italic().to_string()
}

fn command(text: &str) -> String {
    style(text).blue().bold().to_string()
}

fn info(text: &str) -> String {
    style(text).cyan().to_string()
}

struct GenesisKitUI {
    term: Term,
    multi_progress: MultiProgress,
    theme: ColorfulTheme,
}

impl GenesisKitUI {
    fn new() -> Self {
        Self {
            term: Term::stdout(),
            multi_progress: MultiProgress::new(),
            theme: ColorfulTheme::default(),
        }
    }

    fn display_welcome(&self) -> Result<()> {
        self.term.clear_screen()?;
        println!("{}", style(LOGO).cyan().bold());
        println!("{}", heading("Genesis Kit Manager - DevOps Automation Tools"));
        println!("{}\n", style("Version 1.0.0").dim());
        
        println!("{}", heading("Available Commands:"));
        println!("  {} - {}", command("gk repipe"), info("Update Concourse pipelines"));
        println!("  {} - {}", command("gk template"), info("Manage kit template versions"));
        println!("  {} - {}", command("gk ci"), info("Manage CI configuration"));
        println!();
        
        Ok(())
    }

    fn create_progress_bar(&self, len: u64, message: &str) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(len));
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"));
        pb.set_message(message.to_string());
        pb
    }

    async fn repipe_interactive(&self) -> Result<()> {
        println!("\n{}\n", heading("🔄 PIPELINE UPDATE CONFIGURATION"));

        // Select environment
        let environments = vec!["sandbox", "dev", "staging", "prod"];
        let env_selection = Select::with_theme(&self.theme)
            .with_prompt(&param("Select target environment"))
            .items(&environments)
            .default(0)
            .interact()?;

        // Select kits to update
        let available_kits = vec!["shield-v2", "vault-v2", "bosh-v2", "concourse-v6"];
        let kit_selections = MultiSelect::with_theme(&self.theme)
            .with_prompt(&param("Select kits to update (Space to select, Enter to confirm)"))
            .items(&available_kits)
            .defaults(&[true, true, true, true])
            .interact()?;

        // Confirmation for production
        if environments[env_selection] == "prod" {
            let confirm = Confirm::with_theme(&self.theme)
                .with_prompt(&style("⚠️  You're updating production pipelines. Are you sure?").red().bold().to_string())
                .default(false)
                .interact()?;

            if !confirm {
                println!("{}", style("Pipeline update cancelled.").yellow());
                return Ok(());
            }
        }

        println!("\n{}", heading("🚀 UPDATING PIPELINES"));

        for &idx in kit_selections.iter() {
            let kit_name = available_kits[idx];
            let pb = self.create_progress_bar(100, &format!("Updating {} pipeline", kit_name));
            
            for i in 0..100 {
                pb.inc(1);
                thread::sleep(Duration::from_millis(20));
                
                match i {
                    20 => pb.set_message(format!("{} (validating config)", kit_name)),
                    50 => pb.set_message(format!("{} (updating pipeline)", kit_name)),
                    80 => pb.set_message(format!("{} (verifying)", kit_name)),
                    _ => {}
                }
            }
            pb.finish_with_message(format!("✓ {} pipeline updated", kit_name));
        }

        println!("\n{}", style("✨ Pipeline update completed successfully!").green().bold());
        Ok(())
    }

    async fn manage_template_version(&self) -> Result<()> {
        println!("\n{}\n", heading("📋 TEMPLATE VERSION MANAGEMENT"));

        // Select kit to update
        let kits = vec!["shield-v2", "vault-v2", "bosh-v2", "concourse-v6"];
        let kit = Select::with_theme(&self.theme)
            .with_prompt(&param("Select kit to update"))
            .items(&kits)
            .interact()?;

        // Input new version
        let current_version = "2.0.0"; // This would be fetched from the kit
        println!("{} {}", info("Current template version:"), style(current_version).green());
        
        let new_version: String = Input::with_theme(&self.theme)
            .with_prompt(&param("Enter new template version"))
            .validate_with(|input: &String| -> Result<(), &str> {
                Version::parse(input).map_err(|_| "Please enter a valid semantic version (e.g., 2.1.0)")?;
                Ok(())
            })
            .interact_text()?;

        println!("\n{}", heading("🔄 UPDATING TEMPLATE VERSION"));
        
        let pb = self.create_progress_bar(100, "Updating template version");
        for i in 0..100 {
            pb.inc(1);
            thread::sleep(Duration::from_millis(20));
            
            match i {
                30 => pb.set_message("Validating template compatibility..."),
                60 => pb.set_message("Updating dependencies..."),
                90 => pb.set_message("Regenerating configurations..."),
                _ => {}
            }
        }
        pb.finish_with_message("✓ Template version updated successfully");

        // Display update summary
        println!("\n{}", heading("📊 UPDATE SUMMARY"));
        println!("Kit:              {}", style(kits[kit]).green());
        println!("Previous Version: {}", style(current_version).yellow());
        println!("New Version:      {}", style(new_version).green());

        Ok(())
    }

    async fn manage_ci(&self) -> Result<()> {
        println!("\n{}\n", heading("🔧 CI CONFIGURATION"));

        let actions = vec!["View Status", "Update Configuration", "Trigger Build", "View Logs"];
        let action = Select::with_theme(&self.theme)
            .with_prompt(&param("Select CI action"))
            .items(&actions)
            .interact()?;

        match action {
            0 => { // View Status
                println!("\n{}", heading("📊 CI STATUS"));
                let status_table = Table::new(vec![
                    KitStatus {
                        name: "shield-v2".into(),
                        version: "2.3.0".into(),
                        template_version: "2.0.0".into(),
                        ci_status: style("Passing").green().to_string(),
                    },
                    KitStatus {
                        name: "vault-v2".into(),
                        version: "2.1.0".into(),
                        template_version: "2.0.0".into(),
                        ci_status: style("Running").yellow().to_string(),
                    },
                    KitStatus {
                        name: "bosh-v2".into(),
                        version: "2.4.1".into(),
                        template_version: "2.0.0".into(),
                        ci_status: style("Passing").green().to_string(),
                    },
                ]).to_string();

                println!("{}", status_table);
            },
            1 => { // Update Configuration
                let kits = vec!["shield-v2", "vault-v2", "bosh-v2"];
                let kit = Select::with_theme(&self.theme)
                    .with_prompt(&param("Select kit to configure"))
                    .items(&kits)
                    .interact()?;

                println!("\n{}", heading("🔄 UPDATING CI CONFIGURATION"));
                let pb = self.create_progress_bar(100, "Updating CI config");
                for i in 0..100 {
                    pb.inc(1);
                    thread::sleep(Duration::from_millis(20));
                }
                pb.finish_with_message("✓ CI configuration updated");
            },
            2 => { // Trigger Build
                let kits = vec!["shield-v2", "vault-v2", "bosh-v2"];
                let kit = Select::with_theme(&self.theme)
                    .with_prompt(&param("Select kit to build"))
                    .items(&kits)
                    .interact()?;

                println!("\n{}", style("🚀 Triggering CI build...").cyan().bold());
                thread::sleep(Duration::from_secs(2));
                println!("{}", style("✓ Build triggered successfully!").green());
            },
            3 => { // View Logs
                let kits = vec!["shield-v2", "vault-v2", "bosh-v2"];
                let kit = Select::with_theme(&self.theme)
                    .with_prompt(&param("Select kit to view logs"))
                    .items(&kits)
                    .interact()?;

                println!("\n{}", heading("📜 RECENT CI LOGS"));
                // Simulated log output
                println!("{}", style("Fetching latest CI logs...").dim());
                thread::sleep(Duration::from_secs(1));
                println!("{}", style("==> Running tests...").cyan());
                println!("{}", style("==> All tests passed").green());
                println!("{}", style("==> Building release...").cyan());
                println!("{}", style("==> Release built successfully").green());
            },
            _ => unreachable!(),
        }

        Ok(())
    }
}

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
            println!("{}", style("Please specify a command. Use --help for usage information.").yellow());
        }
    }

    Ok(())
}