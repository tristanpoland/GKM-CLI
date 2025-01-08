use console::style;

pub fn heading(text: &str) -> String {
    style(text).magenta().bold().to_string()
}

pub fn param(text: &str) -> String {
    style(text).yellow().italic().to_string()
}

pub fn command(text: &str) -> String {
    style(text).blue().bold().to_string()
}

pub fn info(text: &str) -> String {
    style(text).cyan().to_string()
}

pub fn style_logo(text: &str) -> String {
    style(text).cyan().bold().to_string()
}

pub fn style_version(text: &str) -> String {
    style(text).dim().to_string()
}