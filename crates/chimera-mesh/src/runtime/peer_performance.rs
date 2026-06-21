use super::*;

impl MeshRuntime {
    pub fn update_peer_performance(
        &mut self,
        performance: &[MeshPeerPerformance],
    ) -> Result<(), String> {
        let mut validated = Vec::with_capacity(performance.len());
        for item in performance {
            item.validate()?;
            validate_runtime_node_id(&item.node_id, "mesh peer performance node_id")?;
            if !self.peers.contains_key(&item.node_id) {
                return Err("mesh peer performance references unknown node".to_string());
            }
            validated.push((item.node_id.clone(), item.latency_ms, item.throughput_mbps));
        }
        for (node_id, latency_ms, throughput_mbps) in validated {
            let peer = self
                .peers
                .get_mut(&node_id)
                .ok_or_else(|| "mesh peer performance references unknown node".to_string())?;
            if let Some(latency_ms) = latency_ms {
                peer.latency_ms = Some(latency_ms);
            }
            if let Some(throughput_mbps) = throughput_mbps {
                peer.throughput_mbps = Some(throughput_mbps);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_peer_performance_updates_known_peer_and_rejects_unknown_peer() -> Result<(), String> {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery(
            "seed-b",
            &[MeshDiscoveryRecord {
                node_id: "node-a".to_string(),
                endpoint: "198.51.100.31:443".to_string(),
                region: "eu".to_string(),
                load_score: 20,
                reliability_score: 90,
            }],
        )?;

        runtime.update_peer_performance(&[MeshPeerPerformance {
            node_id: "node-a".to_string(),
            latency_ms: Some(30),
            throughput_mbps: Some(400),
        }])?;

        let peer = runtime
            .peer_snapshot()
            .into_iter()
            .find(|peer| peer.node_id == "node-a")
            .ok_or_else(|| "peer missing".to_string())?;
        assert_eq!(peer.latency_ms, Some(30));
        assert_eq!(peer.throughput_mbps, Some(400));

        let error = match runtime.update_peer_performance(&[MeshPeerPerformance {
            node_id: "missing".to_string(),
            latency_ms: Some(50),
            throughput_mbps: None,
        }]) {
            Ok(_) => return Err("unknown peer performance update must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("unknown node"));
        Ok(())
    }

    #[test]
    fn update_peer_performance_is_atomic_when_batch_contains_unknown_peer() -> Result<(), String> {
        let mut runtime = MeshRuntime::bootstrap("cef-public", "seed-a")?;
        runtime.merge_discovery(
            "seed-b",
            &[MeshDiscoveryRecord {
                node_id: "node-a".to_string(),
                endpoint: "198.51.100.31:443".to_string(),
                region: "eu".to_string(),
                load_score: 20,
                reliability_score: 90,
            }],
        )?;

        let error = match runtime.update_peer_performance(&[
            MeshPeerPerformance {
                node_id: "node-a".to_string(),
                latency_ms: Some(30),
                throughput_mbps: Some(400),
            },
            MeshPeerPerformance {
                node_id: "missing".to_string(),
                latency_ms: Some(50),
                throughput_mbps: None,
            },
        ]) {
            Ok(_) => return Err("mixed-validity performance batch must fail".to_string()),
            Err(error) => error,
        };
        assert!(error.contains("unknown node"));

        let peer = runtime
            .peer_snapshot()
            .into_iter()
            .find(|peer| peer.node_id == "node-a")
            .ok_or_else(|| "peer missing".to_string())?;
        assert_eq!(peer.latency_ms, None);
        assert_eq!(peer.throughput_mbps, None);
        Ok(())
    }
}
