#![forbid(unsafe_code)]

use std::env;

use chimera_carrier::peer_egress::options::Options;
use chimera_carrier::peer_egress::modes;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    };
    let result = match options.mode {
        chimera_carrier::peer_egress::options::Mode::Vps => modes::run_vps(options),
        chimera_carrier::peer_egress::options::Mode::Laptop => modes::run_laptop(options),
        chimera_carrier::peer_egress::options::Mode::Bench => modes::run_bench(options),
        chimera_carrier::peer_egress::options::Mode::Echo => modes::run_echo(options),
        chimera_carrier::peer_egress::options::Mode::Probe => modes::run_probe(options),
        chimera_carrier::peer_egress::options::Mode::DownloadEcho => modes::run_download_echo(options),
        chimera_carrier::peer_egress::options::Mode::DownloadProbe => modes::run_download_probe(options),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
