use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = oss_spec::cli::Cli::parse();
    oss_spec::output::init(cli.debug)?;
    oss_spec::run(cli).await
}
