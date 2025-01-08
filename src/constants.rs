pub const LOGO: &str = r#"
 ██████╗ ██╗  ██╗███╗   ███╗
██╔════╝ ██║ ██╔╝████╗ ████║
██║  ███╗█████╔╝ ██╔████╔██║
██║   ██║██╔═██╗ ██║╚██╔╝██║
╚██████╔╝██║  ██╗██║ ╚═╝ ██║
 ╚═════╝ ╚═╝  ╚═╝╚═╝     ╚═╝
    Concept-0.0.1-alpha
"#;

pub const AVAILABLE_KITS: &[&str] = &["shield-v2", "vault-v2", "bosh-v2", "concourse-v6"];
pub const ENVIRONMENTS: &[&str] = &["sandbox", "dev", "staging", "prod"];

// src/types.rs
use tabled::Tabled;

#[derive(Debug, Tabled)]
pub struct KitStatus {
    #[tabled(rename = "Kit Name")]
    pub name: String,
    #[tabled(rename = "Version")]
    pub version: String,
    #[tabled(rename = "Template Version")]
    pub template_version: String,
    #[tabled(rename = "CI Status")]
    pub ci_status: String,
}