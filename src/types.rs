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