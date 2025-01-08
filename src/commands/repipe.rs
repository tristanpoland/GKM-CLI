// src/commands/repipe.rs
use anyhow::Result;
use dialoguer::{MultiSelect, Select, Confirm};
use std::{thread, time::Duration};
use crate::{
    ui::GenesisKitUI,
    constants::{AVAILABLE_KITS, ENVIRONMENTS},
    ui::styles::*,
    ui::progress::create_progress_bar,
};
use console::style;

impl GenesisKitUI {
    pub async fn repipe_interactive(&self) -> Result<()> {
        println!("\n{}\n", heading("🔄 PIPELINE UPDATE CONFIGURATION"));

        let env_selection = Select::with_theme(&self.theme)
            .with_prompt(&param("Select target environment"))
            .items(ENVIRONMENTS)
            .default(0)
            .interact()?;

        let kit_selections = MultiSelect::with_theme(&self.theme)
            .with_prompt(&param("Select kits to update (Space to select, Enter to confirm)"))
            .items(AVAILABLE_KITS)
            .defaults(&[true, true, true, true])
            .interact()?;

        if ENVIRONMENTS[env_selection] == "prod" {
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
            let kit_name = AVAILABLE_KITS[idx];
            let pb = create_progress_bar(&self.multi_progress, 100, &format!("Updating {} pipeline", kit_name));
            
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
}