use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn step(num: usize, total: usize, msg: &str) {
    println!(
        "{} {}",
        style(format!("[{}/{}]", num, total)).bold().cyan(),
        msg
    );
}

pub fn success(msg: &str) {
    println!("{} {}", style("✓").bold().green(), msg);
}

pub fn error(msg: &str) {
    eprintln!("{} {}", style("✗").bold().red(), msg);
}

pub fn warning(msg: &str) {
    eprintln!("{} {}", style("!").bold().yellow(), msg);
}

pub fn info(msg: &str) {
    println!("{} {}", style("→").bold().blue(), msg);
}

pub fn header(msg: &str) {
    println!("\n{}", style(msg).bold().underlined());
}
