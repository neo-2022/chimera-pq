#![no_main]

use chimera_session::Frame;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = Frame::decode(data);
});
