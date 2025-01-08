use anyhow::Result;
use dialoguer::Select;
use std::{thread, time::Duration};
use tabled::Table;
use console::style;
use crate::{
    ui::GenesisKitUI,
    types::KitStatus,
    constants::AVAILABLE_KITS,
    ui::styles::*,
    ui::progress::create_progress_bar,
};


impl GenesisKitUI {
    pub async fn manage_ci(&self) -> Result<()> {
        println!("\n{}\n", heading("🔧 CI CONFIGURATION"));

        let actions = vec!["View Status", "Update Configuration", "Trigger Build", "View Logs"];
        let action = Select::with_theme(&self.theme)
            .with_prompt(&param("Select CI action"))
            .items(&actions)
            .interact()?;

        match action {
            0 => self.view_ci_status(),
            1 => self.update_ci_config().await?,
            2 => self.trigger_ci_build(),
            3 => self.view_ci_logs(),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn view_ci_status(&self) {
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
    }

    async fn update_ci_config(&self) -> Result<()> {
        let kit = Select::with_theme(&self.theme)
            .with_prompt(&param("Select kit to configure"))
            .items(AVAILABLE_KITS)
            .interact()?;

        println!("\n{}", heading("🔄 UPDATING CI CONFIGURATION"));
        let pb = create_progress_bar(&self.multi_progress, 100, "Updating CI config");
        for i in 0..100 {
            pb.inc(1);
            thread::sleep(Duration::from_millis(20));
        }
        pb.finish_with_message("✓ CI configuration updated");
        Ok(())
    }

    fn trigger_ci_build(&self) {
        let kit = Select::with_theme(&self.theme)
            .with_prompt(&param("Select kit to build"))
            .items(AVAILABLE_KITS)
            .interact()
            .unwrap();

        println!("\n{}", style("🚀 Triggering CI build...").cyan().bold());
        thread::sleep(Duration::from_secs(2));
        println!("{}", style("✓ Build triggered successfully!").green());
    }

    fn view_ci_logs(&self) {
        let kit = Select::with_theme(&self.theme)
            .with_prompt(&param("Select kit to view logs"))
            .items(AVAILABLE_KITS)
            .interact()
            .unwrap();

        println!("\n{}", heading("📜 RECENT CI LOGS"));
        println!("{}", style("Fetching latest CI logs...").dim());
        thread::sleep(Duration::from_secs(1));
        println!("{}", style("==> Running tests...").cyan());
        println!("{}", style("==> All tests passed").green());
        println!("{}", style("==> Building release...").cyan());
        println!("{}", style("==> Release built successfully").green());
    }
}