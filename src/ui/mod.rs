pub mod styles;
pub mod progress;

use console::Term;
use dialoguer::theme::ColorfulTheme;
use indicatif::MultiProgress;
use crate::constants::LOGO;
use anyhow::Result;
use self::styles::*;

pub struct GenesisKitUI {
    pub term: Term,
    pub multi_progress: MultiProgress,
    pub theme: ColorfulTheme,
}

impl GenesisKitUI {
    pub fn new() -> Self {
        Self {
            term: Term::stdout(),
            multi_progress: MultiProgress::new(),
            theme: ColorfulTheme::default(),
        }
    }

    pub fn display_welcome(&self) -> Result<()> {
        self.term.clear_screen()?;
        println!("{}", style_logo(LOGO));
        println!("{}", heading("Genesis Kit Manager - DevOps Automation Tools"));
        println!("{}\n", style_version("Version 1.0.0"));
        
        println!("{}", heading("Available Commands:"));
        println!("  {} - {}", command("gk repipe"), info("Update Concourse pipelines"));
        println!("  {} - {}", command("gk template"), info("Manage kit template versions"));
        println!("  {} - {}", command("gk ci"), info("Manage CI configuration"));
        println!();
        
        Ok(())
    }
}