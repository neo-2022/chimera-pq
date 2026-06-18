use crate::peer_egress::options::Options;

impl std::fmt::Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("mode", &self.mode)
            .field("local_listen", &self.local_listen)
            .field("peer_listen", &self.peer_listen)
            .field("state_file", &self.state_file)
            .field("server", &self.server)
            .field("token", &"<redacted>")
            .field("pool", &self.pool)
            .field("bench_bytes", &self.bench_bytes)
            .field("target", &self.target)
            .field("connect_timeout_ms", &self.connect_timeout_ms)
            .field("min_throughput_mib_s", &self.min_throughput_mib_s)
            .field("connections", &self.connections)
            .field("aead", &self.aead)
            .field("reverse_connect", &self.reverse_connect)
            .field("allow_pool_transit", &self.allow_pool_transit)
            .field("allow_bound_transit", &self.allow_bound_transit)
            .field(
                "transit_lane_bindings_file",
                &self.transit_lane_bindings_file,
            )
            .field("transit_payload_bytes", &self.transit_payload_bytes)
            .field("transit_packet_number", &self.transit_packet_number)
            .field("transit_route_id", &self.transit_route_id)
            .field("transit_lane_index", &self.transit_lane_index)
            .finish()
    }
}
