use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = oss_spec::cli::Cli::parse();
    if let Err(e) = oss_spec::output::init(cli.debug) {
        eprintln!("failed to initialise logging: {e}");
        std::process::exit(1);
    }
    if let Err(e) = oss_spec::run(cli).await {
        oss_spec::output::error(&format!("{e:#}"));
        let log_path = oss_spec::output::log_path();
        oss_spec::output::info(&format!("debug log: {}", log_path.display()));
        std::process::exit(1);
    }
}
