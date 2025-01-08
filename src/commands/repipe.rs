//---------------------------------------------------------------------------//
// Repipe command implementation                                             //
// This is a full reimplementation of the repipe command in Rust for         //
// the GKM CLI tool.                                                         //
//---------------------------------------------------------------------------//
// Authors: Tristan J. Poland                                                //
//---------------------------------------------------------------------------//

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    fs,
};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context, bail};
use tempfile::tempdir;
use log::{info, warn, debug, error};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::PermissionsExt;
use env_logger;
use crate::{
    ui::GenesisKitUI,
};

/// Represents the command-line options for the repipe command
#[derive(Debug, Default)]
pub struct RepipeOptions {
    /// Number of times -v flag is specified (0-2)
    /// - 1: Normal validation
    /// - 2: Strict validation
    pub validate: u8,

    /// Number of times -n flag is specified (0-2)
    /// - 1: Dry run with output to stdout
    /// - 2: Show commands that would be run
    pub dry_run: u8,

    /// Whether to pause the pipeline after uploading (-P flag)
    pub pause: bool,

    /// Pipeline visibility setting (-X or -H flags)
    /// - Some(true): Expose pipeline (-X)
    /// - Some(false): Hide pipeline (-H)
    /// - None: Use setting from meta.exposed
    pub expose: Option<bool>,

    /// Number of times -o flag is specified (0-2)
    /// - 1: Open after applying changes
    /// - 2: Only open, don't apply changes
    pub open_browser: u8,

    /// Whether to run in non-interactive mode (-y flag)
    pub yes: bool,

    /// Custom path to fly executable (--fly option)
    pub fly_path: Option<String>,
}

/// # About
/// Represents the metadata structure from the pipeline configuration
/// 
/// ## Fields
/// 
/// target - Target Concourse instance
/// url - Concourse API URL
/// team - Concourse team name
/// pipeline - Pipeline name (preferred over 'name' field)
/// name - Alternative pipeline name (used if 'pipeline' is not set)
/// exposed - Whether the pipeline should be exposed publicly
#[derive(Debug, Serialize, Deserialize)]
struct PipelineMeta {
    target:   Option<String>,
    url:      Option<String>,
    team:     Option<String>,
    pipeline: Option<String>,
    name:     Option<String>,
    exposed:  Option<bool>,
}

/// # About
/// Main struct for handling the repipe command functionality
/// 
/// ## Fields
/// 
/// options - Command line options
/// base_dir - Base directory where the command is run
/// settings_file - Path to the settings file (settings.yml or settings-{target}.yml)
/// meta - Parsed pipeline metadata
/// target - Target Concourse instance
/// pipeline - Pipeline name
pub struct RepipeCommand {
    options:       RepipeOptions,
    base_dir:      PathBuf,
    settings_file: String,
    meta:          Option<PipelineMeta>,
    target:        String,
    pipeline:      String,
}

impl RepipeCommand {
    pub fn new(options: RepipeOptions) -> Result<Self> {
        info!("Initializing RepipeCommand with options: {:?}", options);
        let base_dir = env::current_dir()?;
        debug!("Base directory set to: {}", base_dir.display());
        
        Ok(Self {
            options,
            base_dir,
            settings_file: String::from("settings.yml"),
            meta: None,
            target: String::new(),
            pipeline: String::new(),
        })
    }

    fn check_requirements(&self) -> Result<()> {
        info!("Checking system requirements...");
        
        // Check for required commands in order of importance
        info!("Checking for required command-line tools...");
        
        debug!("Verifying 'which' command availability");
        Command::new("which")
            .arg("which")
            .output()
            .map_err(|e| {
                error!("Critical: 'which' command is not available: {}", e);
                anyhow::anyhow!("'which' command is not available - this is required for system checks")
            })?;
            
        debug!("Verifying spruce installation");
        if let Err(e) = self.check_command("spruce", Some("https://github.com/geofffranks/spruce/releases")) {
            error!("Failed to verify spruce installation: {}", e);
            return Err(e);
        }
        
        debug!("Verifying jq installation");
        if let Err(e) = self.check_command("jq", None) {
            error!("Failed to verify jq installation: {}", e);
            return Err(e);
        }
        
        // Check fly command
        if let Some(fly_path) = &self.options.fly_path {
            debug!("Checking custom fly path: {}", fly_path);
            let metadata = fs::metadata(fly_path)?;
            #[cfg(unix)]
            let is_executable = metadata.permissions().mode() & 0o111 != 0;
            #[cfg(not(unix))]
            let is_executable = metadata.is_file();

            if !is_executable {
                error!("Specified fly path '{}' is not executable", fly_path);
                bail!("Specified fly path '{}' is not executable", fly_path);
            }
        } else {
            debug!("Verifying fly installation in PATH");
            self.check_command("fly", None)?;
        }

        info!("All system requirements verified successfully");
        Ok(())
    }

    fn check_command(&self, cmd: &str, url: Option<&str>) -> Result<()> {
        debug!("Checking for command: {}", cmd);
        let which_result = Command::new("which")
            .arg(cmd)
            .output()
            .map_err(|e| {
                error!("Failed to execute 'which' command: {}", e);
                e
            })?;
            
        if which_result.status.success() {
            let path = String::from_utf8_lossy(&which_result.stdout);
            debug!("Command '{}' found at: {}", cmd, path.trim());
            Ok(())
        } else {
            let msg = format!("Required command '{}' is not installed or not found in PATH", cmd);
            error!("{}", msg);
            if let Some(download_url) = url {
                error!("You can download '{}' from: {}", cmd, download_url);
                bail!("{}\nPlease download it from: {}", msg, download_url);
            } else {
                error!("Please install '{}' using your system's package manager", cmd);
                bail!("{}\nPlease install using your system's package manager", msg);
            }
        }
    }

    fn find_settings_file(&mut self) -> Result<()> {
        info!("Looking for settings file...");
        
        // Check for target-specific settings file
        if let Ok(target) = env::var("CONCOURSE_TARGET") {
            debug!("Found CONCOURSE_TARGET environment variable: {}", target);
            let target_file = format!(
                "settings-{}.yml", 
                target.replace('/', "-").replace(' ', "_")
            );
            debug!("Checking for target-specific settings file: {}", target_file);
            if Path::new(&target_file).exists() {
                info!("Found target-specific settings file");
                self.settings_file = target_file;
            }
        }

        if !Path::new(&self.settings_file).exists() {
            error!("Settings file not found: {}", self.settings_file);
            bail!("Missing local settings in ci/settings.yml!");
        }

        info!("Using settings file: {}", self.settings_file);
        Ok(())
    }

    fn merge_pipeline_config(&self) -> Result<String> {
        info!("Merging pipeline configuration...");
        let temp_dir = tempdir()?;
        debug!("Created temporary directory: {}", temp_dir.path().display());
        
        // Run optional build scripts
        if Path::new("scripts/build-test-jobs").exists() {
            info!("Running build-test-jobs script");
            Command::new("./scripts/build-test-jobs").status()?;
        }
        if Path::new("scripts/build-upstream-jobs").exists() {
            info!("Running build-upstream-jobs script");
            Command::new("./scripts/build-upstream-jobs").status()?;
        }

        // Merge pipeline files using spruce
        debug!("Running spruce merge command");
        let output = Command::new("spruce")
            .args(&["merge", "--fallback-append", "pipeline/base.yml"])
            .arg(format!("{}/pipeline/*/*.yml", self.base_dir.display()))
            .arg(&self.settings_file)
            .output()?;

        if !output.status.success() {
            error!("Pipeline merge failed");
            bail!("Failed to merge pipeline configuration");
        }

        let config = String::from_utf8(output.stdout)?;
        let deploy_path = temp_dir.path().join(".deploy.yml");
        fs::write(&deploy_path, &config)?;
        info!("Pipeline configuration merged successfully");
        debug!("Deploy configuration written to: {}", deploy_path.display());

        Ok(config)
    }

    fn extract_meta(&mut self, config: &str) -> Result<()> {
        info!("Extracting pipeline metadata...");
        
        // Extract meta information using spruce
        debug!("Running spruce merge for meta extraction");
        let mut child = Command::new("spruce")
            .args(&["merge", "--skip-eval", "--cherry-pick", "meta"])
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        // Write config to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(config.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            error!("Failed to extract meta information");
            bail!("Failed to extract meta information");
        }

        // Parse meta JSON and store it
        debug!("Parsing meta information JSON");
        let meta_json = String::from_utf8(output.stdout)?;
        self.meta = Some(serde_json::from_str(&meta_json)?);

        // Extract and validate target and pipeline name
        if let Some(meta) = &self.meta {
            debug!("Extracted meta: {:?}", meta);
            
            // Get target from meta or environment
            self.target = meta.target.clone()
                .or_else(|| {
                    let env_target = env::var("CONCOURSE_TARGET").ok();
                    if let Some(ref t) = env_target {
                        debug!("Using target from environment: {}", t);
                    }
                    env_target
                })
                .context("Settings file missing meta.target value")?;

            // Get pipeline name from meta
            self.pipeline = meta.pipeline.clone()
                .or_else(|| meta.name.clone())
                .context("Missing pipeline name in settings!")?;
            
            info!("Target set to: {}", self.target);
            info!("Pipeline name set to: {}", self.pipeline);
        }

        Ok(())
    }

    fn validate_target(&self) -> Result<()> {
        info!("Validating target configuration...");
        
        // Read flyrc config
        let home = env::var("HOME")?;
        let flyrc_path = format!("{}/.flyrc", home);
        debug!("Reading flyrc from: {}", flyrc_path);
        let flyrc = fs::read_to_string(&flyrc_path)?;
        let flyrc_json: serde_json::Value = serde_json::from_str(&flyrc)?;

        if let Some(meta) = &self.meta {
            // Validate target URL if specified
            if let Some(pipeline_url) = &meta.url {
                debug!("Validating target URL");
                let target_url = flyrc_json["targets"][&self.target]["api"]
                    .as_str()
                    .context("Could not find target URL in .flyrc")?;
                
                if pipeline_url != target_url {
                    error!("Target URL mismatch: {} != {}", pipeline_url, target_url);
                    bail!("Target URL mismatch");
                }
                debug!("Target URL validated successfully");
            }

            // Validate team if specified
            if let Some(pipeline_team) = &meta.team {
                debug!("Validating target team");
                let target_team = flyrc_json["targets"][&self.target]["team"]
                    .as_str()
                    .context("Could not find target team in .flyrc")?;
                
                if pipeline_team != target_team {
                    error!("Target team mismatch: {} != {}", pipeline_team, target_team);
                    bail!("Target team mismatch");
                }
                debug!("Target team validated successfully");
            }
        }

        info!("Target configuration validated successfully");
        Ok(())
    }

    pub fn execute(&mut self) -> Result<()> {
        info!("Starting pipeline deployment process");
        
        // Initial setup and validation
        self.check_requirements()?;
        self.find_settings_file()?;

        // Merge pipeline configuration and validate
        let config = self.merge_pipeline_config()?;
        self.extract_meta(&config)?;
        self.validate_target()?;

        let fly_cmd = self.options.fly_path.clone()
            .unwrap_or_else(|| String::from("fly"));
        debug!("Using fly command: {}", fly_cmd);

        // Handle different modes of operation based on options
        if self.options.validate > 0 {
            info!("Running in validation mode (level: {})", self.options.validate);
            let mut cmd = Command::new(&fly_cmd);
            cmd.args(&["--target", &self.target, "validate-pipeline"]);
            if self.options.validate >= 2 {
                debug!("Using strict validation");
                cmd.arg("--strict");
            }
            cmd.arg("--config").arg(".deploy.yml");
            cmd.status()?;
        } else if self.options.dry_run > 0 {
            info!("Running in dry-run mode");
            println!("{}", fs::read_to_string(".deploy.yml")?);
        } else {
            info!("Running in normal execution mode");
            
            // Set pipeline
            info!("Setting pipeline configuration");
            let mut set_cmd = Command::new(&fly_cmd);
            set_cmd.args(&[
                "--target", &self.target,
                "set-pipeline",
                "--pipeline", &self.pipeline,
                "--config", ".deploy.yml",
            ]);
            if self.options.yes {
                debug!("Using non-interactive mode");
                set_cmd.arg("--non-interactive");
            }
            set_cmd.status()?;

            // Handle pause/unpause
            let pause_cmd = if self.options.pause { "pause" } else { "unpause" };
            info!("{}ing pipeline", pause_cmd);
            Command::new(&fly_cmd)
                .args(&[
                    "--target", &self.target,
                    &format!("{}-pipeline", pause_cmd),
                    "--pipeline", &self.pipeline,
                ])
                .status()?;

            // Handle expose/hide
            let expose = self.options.expose.unwrap_or_else(|| {
                self.meta.as_ref()
                    .and_then(|m| m.exposed)
                    .unwrap_or(false)
            });
            let expose_cmd = if expose { "expose" } else { "hide" };
            info!("{}ing pipeline", expose_cmd);
            Command::new(&fly_cmd)
                .args(&[
                    "--target", &self.target,
                    &format!("{}-pipeline", expose_cmd),
                    "--pipeline", &self.pipeline,
                ])
                .status()?;
        }

        // Open pipeline in browser if requested
        if self.options.open_browser > 0 {
            info!("Opening pipeline in browser");
            let url = format!(
                "{}/teams/{}/pipelines/{}", 
                self.meta.as_ref().and_then(|m| m.url.as_ref()).unwrap_or(&String::new()),
                self.meta.as_ref().and_then(|m| m.team.as_ref()).unwrap_or(&String::new()),
                self.pipeline
            );
            debug!("Pipeline URL: {}", url);

            if let Err(e) = open::that(&url) {
                warn!("Could not open browser automatically: {}", e);
                println!("Could not open the browser automatically: {}", e);
                println!("\nHere's the URL you can open manually:");
                println!("  {}", url);
            }
        }

        info!("Pipeline deployment process completed successfully");
        Ok(())
    }
}

impl GenesisKitUI {
    pub fn repipe_interactive(&self) {
        // Initialize env_logger if it hasn't been initialized yet
        if env_logger::try_init().is_ok() {
            // Set RUST_LOG if not already set
            if env::var("RUST_LOG").is_err() {
                env::set_var("RUST_LOG", "info");
            }
        }

        info!("Starting interactive repipe process");
        let mut repipe = RepipeCommand::new(RepipeOptions::default())
            .expect("Failed to create RepipeCommand instance");
        if let Err(e) = repipe.execute() {
            error!("Repipe execution failed: {}", e);
        }
    }
}