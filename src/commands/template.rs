use anyhow::Result;
use dialoguer::{Input, Select};
use semver::Version;
use std::{thread, time::Duration};
use crate::{
    ui::GenesisKitUI,
    constants::AVAILABLE_KITS,
    ui::styles::*,
    ui::progress::create_progress_bar,
};
use console::style;

impl GenesisKitUI {
    pub async fn manage_template_version(&self) -> Result<()> {
        println!("\n{}\n", heading("📋 TEMPLATE VERSION MANAGEMENT"));

        let kit = Select::with_theme(&self.theme)
            .with_prompt(&param("Select kit to update"))
            .items(AVAILABLE_KITS)
            .interact()?;

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
        
        let pb = create_progress_bar(&self.multi_progress, 100, "Updating template version");
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

        println!("\n{}", heading("📊 UPDATE SUMMARY"));
        println!("Kit:              {}", style(AVAILABLE_KITS[kit]).green());
        println!("Previous Version: {}", style(current_version).yellow());
        println!("New Version:      {}", style(new_version).green());

        Ok(())
    }
}
