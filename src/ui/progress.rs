use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub fn create_progress_bar(multi_progress: &MultiProgress, len: u64, message: &str) -> ProgressBar {
    let pb = multi_progress.add(ProgressBar::new(len));
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap()
        .progress_chars("=>-"));
    pb.set_message(message.to_string());
    pb
}