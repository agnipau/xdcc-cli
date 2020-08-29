use {
    anyhow::{Context, Result},
    std::time::Duration,
    structopt::StructOpt,
    xdcc_cli::{PacksRanges, Xdcc},
};

#[derive(Debug, StructOpt)]
struct Opt {
    /// URL or IP address of the IRC server.
    server: String,

    /// IRC xdcc bot.
    bot: String,

    /// IRC channels.
    channel: String,

    /// Packs ranges. Example: 12-15 17 19-20, expands to 12 13 14 15 17 19 20.
    #[structopt(required = true)]
    packs_ranges: Vec<String>,

    /// IRC server port.
    #[structopt(long, default_value = "6667")]
    port: u16,

    /// Nickname for IRC connection (default: random).
    #[structopt(long)]
    nick: Option<String>,

    /// Don't ask for confirmation before downloading a file.
    #[structopt(short = "y", long)]
    no_confirm: bool,

    /// Be verbose (debug messages). You can also set the RUST_LOG env var for
    /// finer control.
    #[structopt(short = "v", long)]
    verbose: bool,

    /// Path where to download files.
    #[structopt(long, default_value = ".")]
    out_path: String,

    /// DCC request timeout. If 0.0 no timeout is performed.
    #[structopt(long, default_value = "30.0")]
    request_timeout_secs: f64,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    if opt.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    let packs_ranges =
        PacksRanges::from(&opt.packs_ranges.iter().map(|p| &**p).collect::<Vec<_>>());
    let req_timeout = Duration::from_secs_f64(opt.request_timeout_secs);

    let mut xdcc = Xdcc::new(
        opt.nick.as_deref(),
        &opt.server,
        opt.port,
        &opt.bot,
        &opt.channel,
        &packs_ranges,
        req_timeout,
    )
    .context("Failed to create Xdcc instance")?;
    xdcc.download().context("Failed to download file")?;

    Ok(())
}
