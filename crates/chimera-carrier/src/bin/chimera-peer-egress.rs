#![forbid(unsafe_code)]

use std::env;

use chimera_carrier::peer_egress::options::Options;
use chimera_carrier::peer_egress::{modes, node, proof};

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
        chimera_carrier::peer_egress::options::Mode::Node => node::run_node(options),
        chimera_carrier::peer_egress::options::Mode::SideA => modes::run_side_a(options),
        chimera_carrier::peer_egress::options::Mode::SideB => modes::run_side_b(options),
        chimera_carrier::peer_egress::options::Mode::Bench => modes::run_bench(options),
        chimera_carrier::peer_egress::options::Mode::Echo => modes::run_echo(options),
        chimera_carrier::peer_egress::options::Mode::Probe => modes::run_probe(options),
        chimera_carrier::peer_egress::options::Mode::DownloadEcho => {
            modes::run_download_echo(options)
        }
        chimera_carrier::peer_egress::options::Mode::DownloadProbe => {
            modes::run_download_probe(options)
        }
        chimera_carrier::peer_egress::options::Mode::SealedTransitInject => {
            proof::run_sealed_transit_inject(options)
        }
        chimera_carrier::peer_egress::options::Mode::BoundTransitInject => {
            proof::run_bound_transit_inject(options)
        }
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
