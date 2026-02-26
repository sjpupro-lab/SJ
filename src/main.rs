use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name="canvapress", version, about="Canvapress CVP1 encoder/decoder")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Encode { input: String, output: String },
    Decode { input: String, output: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Encode { input, output } => {
            let data = std::fs::read(input)?;
            let raw = canvapress::encode_erase(&data)?;
            std::fs::write(output, raw)?;
        }
        Cmd::Decode { input, output } => {
            let raw = std::fs::read(input)?;
            let data = canvapress::decode_fill(&raw)?;
            std::fs::write(output, data)?;
        }
    }
    Ok(())
}