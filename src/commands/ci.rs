use anyhow::{Result, Context};
use dialoguer::Select;
use std::{thread, time::Duration, process::Command};
use tabled::Table;
use console::style;
use tokio::process::Command as AsyncCommand;
use serde_json::Value;
use crate::{
    ui::GenesisKitUI,
    types::KitStatus,
    constants::AVAILABLE_KITS,
    ui::styles::*,
    ui::progress::create_progress_bar,
};

impl GenesisKitUI {
    pub async fn manage_ci(&self) -> Result<()> {
        // First check if fly CLI is available
        self.check_fly_cli()?;

        println!("\n{}\n", heading("🔧 CI CONFIGURATION"));

        let actions = vec!["View Status", "Update Configuration", "Trigger Build", "View Logs"];
        let action = Select::with_theme(&self.theme)
            .with_prompt(&param("Select CI action"))
            .items(&actions)
            .interact()?;

        match action {
            0 => self.view_ci_status().await?,
            1 => self.update_ci_config().await?,
            2 => self.trigger_ci_build().await?,
            3 => self.view_ci_logs().await?,
            _ => unreachable!(),
        }

        Ok(())
    }

    fn check_fly_cli(&self) -> Result<()> {
        let output = Command::new("fly")
            .arg("--version")
            .output()
            .context("Failed to check fly CLI. Please ensure it's installed and in your PATH")?;

        if !output.status.success() {
            anyhow::bail!("Fly CLI is not properly configured. Please run 'fly login' first");
        }
        Ok(())
    }

    async fn view_ci_status(&self) -> Result<()> {
        println!("\n{}", heading("📊 CI STATUS"));
        
        let mut statuses = Vec::new();
        
        for kit in AVAILABLE_KITS {
            let output = AsyncCommand::new("fly")
                .args(["builds", "-j", &format!("genesis-kits/{}", kit)])
                .output()
                .await
                .context("Failed to fetch build status")?;

            let status = if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let latest_status = stdout.lines().next()
                    .map(|line| line.split_whitespace().nth(2))
                    .flatten()
                    .unwrap_or("unknown");

                match latest_status {
                    "succeeded" => style("Passing").green().to_string(),
                    "failed" => style("Failed").red().to_string(),
                    "started" => style("Running").yellow().to_string(),
                    _ => style("Unknown").dim().to_string(),
                }
            } else {
                style("Error").red().to_string()
            };

            // Fetch pipeline config for version info
            let config_output = AsyncCommand::new("fly")
                .args(["configure", "-t", "genesis-kits", "-j", kit, "--json"])
                .output()
                .await
                .context("Failed to fetch pipeline config")?;

            let config: Value = if config_output.status.success() {
                serde_json::from_slice(&config_output.stdout)
                    .context("Failed to parse pipeline config")?
            } else {
                Value::Null
            };

            let version = config["version"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            let template_version = config["template_version"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            statuses.push(KitStatus {
                name: (*kit).into(),
                version,
                template_version,
                ci_status: status,
            });
        }

        let status_table = Table::new(statuses).to_string();
        println!("{}", status_table);
        Ok(())
    }

    async fn update_ci_config(&self) -> Result<()> {
        let kit = Select::with_theme(&self.theme)
            .with_prompt(&param("Select kit to configure"))
            .items(AVAILABLE_KITS)
            .interact()?;

        let kit_name = AVAILABLE_KITS[kit];
        println!("\n{}", heading("🔄 UPDATING CI CONFIGURATION"));

        let pb = create_progress_bar(&self.multi_progress, 3, "Updating CI config");

        // Download current pipeline config
        pb.set_message("Downloading current pipeline config...");
        let output = AsyncCommand::new("fly")
            .args([
                "get-pipeline",
                "-t", "genesis-kits",
                "-p", kit_name,
            ])
            .output()
            .await
            .context("Failed to fetch pipeline config")?;

        if !output.status.success() {
            anyhow::bail!("Failed to download pipeline configuration");
        }
        pb.inc(1);

        // Save to temporary file
        let config = String::from_utf8_lossy(&output.stdout);
        let temp_file = format!("/tmp/{}-pipeline.yml", kit_name);
        std::fs::write(&temp_file, config.as_bytes())
            .context("Failed to save pipeline config")?;
        pb.inc(1);

        // Set updated pipeline
        pb.set_message("Uploading new configuration...");
        let set_output = AsyncCommand::new("fly")
            .args([
                "set-pipeline",
                "-t", "genesis-kits",
                "-p", kit_name,
                "-c", &temp_file,
                "--non-interactive",
            ])
            .output()
            .await
            .context("Failed to update pipeline")?;

        if !set_output.status.success() {
            anyhow::bail!("Failed to update pipeline configuration");
        }

        pb.finish_with_message("✓ CI configuration updated");
        Ok(())
    }

    async fn trigger_ci_build(&self) -> Result<()> {
        let kit = Select::with_theme(&self.theme)
            .with_prompt(&param("Select kit to build"))
            .items(AVAILABLE_KITS)
            .interact()?;

        let kit_name = AVAILABLE_KITS[kit];
        println!("\n{}", style("🚀 Triggering CI build...").cyan().bold());

        let output = AsyncCommand::new("fly")
            .args([
                "trigger-job",
                "-t", "genesis-kits",
                "-j", &format!("{}/test-kit", kit_name),
                "--watch",
            ])
            .output()
            .await
            .context("Failed to trigger build")?;

        if output.status.success() {
            println!("{}", style("✓ Build completed successfully!").green());
        } else {
            println!("{}", style("⨯ Build failed").red());
            println!("Build output:\n{}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    async fn view_ci_logs(&self) -> Result<()> {
        let kit = Select::with_theme(&self.theme)
            .with_prompt(&param("Select kit to view logs"))
            .items(AVAILABLE_KITS)
            .interact()?;

        let kit_name = AVAILABLE_KITS[kit];
        println!("\n{}", heading("📜 RECENT CI LOGS"));
        println!("{}", style("Fetching latest CI logs...").dim());

        let output = AsyncCommand::new("fly")
            .args([
                "builds",
                "-t", "genesis-kits",
                "-j", &format!("{}/test-kit", kit_name),
                "--count=1",
                "--json",
            ])
            .output()
            .await
            .context("Failed to fetch build info")?;

        if !output.status.success() {
            anyhow::bail!("Failed to fetch build information");
        }

        let builds: Value = serde_json::from_slice(&output.stdout)
            .context("Failed to parse build info")?;

        if let Some(build) = builds.as_array().and_then(|arr| arr.first()) {
            if let Some(build_id) = build["number"].as_str() {
                let log_output = AsyncCommand::new("fly")
                    .args([
                        "watch",
                        "-t", "genesis-kits",
                        "-j", &format!("{}/test-kit", kit_name),
                        "-b", build_id,
                    ])
                    .output()
                    .await
                    .context("Failed to fetch build logs")?;

                println!("{}", String::from_utf8_lossy(&log_output.stdout));
            }
        }

        Ok(())
    }
}