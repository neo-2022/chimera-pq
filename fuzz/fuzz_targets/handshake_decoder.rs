#![no_main]

use chimera_session::HandshakeMessage;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = HandshakeMessage::decode(data);
});
