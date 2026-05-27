#![no_main]

use chimera_config::RawConfig;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = RawConfig::parse(&text);
});
