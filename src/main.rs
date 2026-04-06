use anyhow::Result;
use apple_health_resonator::cli;
use apple_health_resonator::infra::logging;
use clap::Parser;

fn main() -> Result<()> {
    logging::init_logging()?;
    let cli = cli::args::Cli::parse();
    cli::run(cli)
}
