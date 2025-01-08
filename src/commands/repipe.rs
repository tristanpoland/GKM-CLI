use std::{env, path::{Path, PathBuf}, process::Command, fs};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context, bail};
use log::error;
use crate::GenesisKitUI;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Default)]
pub struct RepipeOptions {
    pub validate: u8,
    pub dry_run: u8,
    pub pause: bool,
    pub expose: Option<bool>,
    pub open_browser: u8,
    pub yes: bool,
    pub fly_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PipelineMeta {
    target: Option<String>,
    url: Option<String>,
    team: Option<String>,
    pipeline: Option<String>,
    name: Option<String>,
    exposed: Option<bool>,
}

pub struct RepipeCommand {
    options: RepipeOptions,
    base_dir: PathBuf,
    settings_file: String,
    meta: Option<PipelineMeta>,
    target: String,
    pipeline: String,
}

impl RepipeCommand {
    pub fn new(options: RepipeOptions) -> Result<Self> {
        let base_dir = Self::find_ci_directory()?;
        env::set_current_dir(&base_dir)?;
        
        Ok(Self {
            options,
            base_dir,
            settings_file: String::from("settings.yml"),
            meta: None,
            target: String::new(),
            pipeline: String::new(),
        })
    }

    fn find_ci_directory() -> Result<PathBuf> {
        let current_dir = env::current_dir()?;
        if current_dir.ends_with("ci") {
            return Ok(current_dir);
        }
        
        let ci_dir = current_dir.join("ci").exists()
            .then_some(current_dir.join("ci"))
            .or_else(|| current_dir.parent().map(|p| p.join("ci")).filter(|p| p.exists()))
            .context("Could not find ci directory")?;
            
        Ok(ci_dir)
    }

    fn check_requirements(&self) -> Result<()> {
        let required_commands = vec![
            ("which", None),
            ("spruce", Some("https://github.com/geofffranks/spruce/releases")),
            ("jq", None),
        ];

        for (cmd, url) in required_commands {
            Command::new("which").arg(cmd).output()
                .map_err(|_| anyhow::anyhow!("'{}' command not found{}", cmd, 
                    url.map(|u| format!("\nDownload from: {}", u))
                        .unwrap_or_else(|| String::from("\nInstall via package manager"))))?;
        }

        if let Some(path) = &self.options.fly_path {
            #[cfg(unix)]
            if fs::metadata(path)?.permissions().mode() & 0o111 == 0 {
                bail!("Specified fly path '{}' is not executable", path);
            }
        } else {
            Command::new("which").arg("fly").output()?;
        }
        Ok(())
    }

    fn find_settings_file(&mut self) -> Result<()> {
        if let Ok(target) = env::var("CONCOURSE_TARGET") {
            let target_file = format!("settings-{}.yml", target.replace('/', "-").replace(' ', "_"));
            if Path::new(&target_file).exists() {
                self.settings_file = target_file;
            }
        }

        if !Path::new(&self.settings_file).exists() {
            bail!("Missing local settings in ci/settings.yml!");
        }
        Ok(())
    }

    fn merge_pipeline_config(&self) -> Result<String> {

        let base_yml = self.base_dir.join("pipeline").join("base.yml");
        if !base_yml.exists() {
            bail!("Missing pipeline/base.yml file");
        }

        let mut yaml_files = vec![base_yml];
        let pipeline_dir = self.base_dir.join("pipeline");
        if pipeline_dir.exists() {
            for entry in fs::read_dir(&pipeline_dir)? {
                let path = entry?.path();
                if path.extension().map_or(false, |ext| ext == "yml") {
                    yaml_files.push(path);
                }
            }
        }

        let output = Command::new("spruce")
            .arg("merge")
            .arg("--fallback-append")
            .args(&yaml_files)
            .arg(&self.settings_file)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to merge pipeline configuration: {}", stderr);
        }

        let yaml_output = String::from_utf8(output.stdout)?;
        // Validate YAML format
        if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&yaml_output) {
            bail!("Invalid YAML output: {}", e);
        }

        // Write the YAML output to .deploy.yml for fly
        fs::write(".deploy.yml", &yaml_output)?;
        
        Ok(yaml_output)
    }

    fn extract_meta(&mut self, config: &str) -> Result<()> {
        let mut child = Command::new("spruce")
            .args(&["merge", "--skip-eval", "--cherry-pick", "meta"])
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(config.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        #[derive(Deserialize)]
        struct MetaWrapper { meta: PipelineMeta }
        
        let wrapper: MetaWrapper = serde_yaml::from_str(&String::from_utf8(output.stdout)?)?;
        self.meta = Some(wrapper.meta);

        if let Some(meta) = &self.meta {
            self.target = meta.target.clone()
                .or_else(|| env::var("CONCOURSE_TARGET").ok())
                .context("Missing target")?;

            self.pipeline = meta.pipeline.clone()
                .or_else(|| meta.name.clone())
                .context("Missing pipeline name")?;
        }
        Ok(())
    }

    fn validate_target(&self) -> Result<()> {
        let flyrc_path = [
            dirs::home_dir().map(|p| p.join(".flyrc")),
            env::var("FLYRC").ok().map(PathBuf::from),
            Some(PathBuf::from(".flyrc")),
        ].into_iter().flatten().find(|p| p.exists())
            .context("Could not find .flyrc file")?;

        let flyrc: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(flyrc_path)?)?;
        let targets = flyrc.get("targets").context("No targets in .flyrc")?;

        if !targets.get(&self.target).is_some() {
            bail!("Target '{}' not found", self.target);
        }

        if let Some(meta) = &self.meta {
            if let Some(url) = &meta.url {
                if url != targets[&self.target]["api"].as_str().unwrap_or_default() {
                    bail!("Target URL mismatch");
                }
            }
            if let Some(team) = &meta.team {
                if team != targets[&self.target]["team"].as_str().unwrap_or_default() {
                    bail!("Target team mismatch");
                }
            }
        }
        Ok(())
    }

    pub fn execute(&mut self) -> Result<()> {
        self.check_requirements()?;
        self.find_settings_file()?;
        
        let config = self.merge_pipeline_config()?;
        self.extract_meta(&config)?;
        self.validate_target()?;

        let fly = self.options.fly_path.clone().unwrap_or_else(|| String::from("fly"));

        match (self.options.validate, self.options.dry_run) {
            (v, 0) if v > 0 => {
                Command::new(&fly)
                    .args(&["--target", &self.target, "validate-pipeline"])
                    .arg(if v >= 2 { "--strict" } else { "" })
                    .arg("--config").arg(".deploy.yml")
                    .status()?;
            },
            (0, d) if d > 0 => println!("{}", fs::read_to_string(".deploy.yml")?),
            _ => {
                // Set pipeline
                Command::new(&fly)
                    .args(&["--target", &self.target, "set-pipeline", "--pipeline", &self.pipeline])
                    .args(&["--config", ".deploy.yml"])
                    .arg(if self.options.yes { "--non-interactive" } else { "" })
                    .status()?;

                // Pause/unpause
                let pause_cmd = if self.options.pause { "pause" } else { "unpause" };
                Command::new(&fly)
                    .args(&["--target", &self.target, &format!("{}-pipeline", pause_cmd)])
                    .args(&["--pipeline", &self.pipeline])
                    .status()?;

                // Expose/hide
                let expose = self.options.expose
                    .unwrap_or_else(|| self.meta.as_ref().and_then(|m| m.exposed).unwrap_or(false));
                Command::new(&fly)
                    .args(&["--target", &self.target])
                    .args(&[if expose { "expose-pipeline" } else { "hide-pipeline" }])
                    .args(&["--pipeline", &self.pipeline])
                    .status()?;
            }
        }

        if self.options.open_browser > 0 {
            if let Some(meta) = &self.meta {
                let url = format!("{}/teams/{}/pipelines/{}", 
                    meta.url.as_ref().unwrap_or(&String::new()),
                    meta.team.as_ref().unwrap_or(&String::new()),
                    self.pipeline
                );
                if let Err(e) = open::that(&url) {
                    println!("Could not open browser automatically: {}\nURL: {}", e, url);
                }
            }
        }
        Ok(())
    }
}

impl GenesisKitUI {
    pub fn repipe_interactive(&self) {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_timestamp(Some(env_logger::TimestampPrecision::Seconds))
            .format_module_path(true)
            .init();

        if let Err(e) = RepipeCommand::new(RepipeOptions::default()).and_then(|mut cmd| cmd.execute()) {
            error!("Repipe failed: {}", e);
        }
    }
}