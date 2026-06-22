#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Node,
    SideA,
    SideB,
    Bench,
    Echo,
    Probe,
    DownloadEcho,
    DownloadProbe,
    SealedTransitInject,
    BoundTransitInject,
}

pub fn parse_mode(value: &str) -> Result<Mode, String> {
    match value {
        "node" | "weave-node" => Ok(Mode::Node),
        "side-a" | "side_a" => Ok(Mode::SideA),
        "side-b" | "side_b" => Ok(Mode::SideB),
        "bench" => Ok(Mode::Bench),
        "echo" => Ok(Mode::Echo),
        "probe" => Ok(Mode::Probe),
        "download-echo" => Ok(Mode::DownloadEcho),
        "download-probe" => Ok(Mode::DownloadProbe),
        "sealed-transit-inject" => Ok(Mode::SealedTransitInject),
        "bound-transit-inject" => Ok(Mode::BoundTransitInject),
        _ => Err(
            "mode must be node, side-a, side-b, bench, echo, probe, download-echo, download-probe, sealed-transit-inject, or bound-transit-inject"
                .to_string(),
        ),
    }
}

pub fn mode_name(mode: &Mode) -> &'static str {
    match mode {
        Mode::Node => "node",
        Mode::SideA => "side-a",
        Mode::SideB => "side-b",
        Mode::Bench => "bench",
        Mode::Echo => "echo",
        Mode::Probe => "probe",
        Mode::DownloadEcho => "download-echo",
        Mode::DownloadProbe => "download-probe",
        Mode::SealedTransitInject => "sealed-transit-inject",
        Mode::BoundTransitInject => "bound-transit-inject",
    }
}
