# CEF Track Report

Status: **PARTIAL / NOT CLOSED**

Truth boundary:
- MVP lab ready: `true`
- Full CEF closed: `false`
- Phase-1 closed: `true`

Block status:
- mesh_runtime: `true`
  checks: code=`true`, api=`true`, tests=`true`
  runtime_wired: `true`, gate_status: `ready`
  mesh_runtime: found paths -> crates/chimera-mesh
- dht_discovery: `true`
  checks: code=`true`, api=`true`, tests=`true`
  runtime_wired: `true`, gate_status: `ready`
  dht_discovery: found paths -> crates/chimera-dht
- distributed_policy_store: `true`
  checks: code=`true`, api=`true`, tests=`true`
  runtime_wired: `true`, gate_status: `ready`
  distributed_policy_store: found paths -> crates/chimera-dps
- cooperative_relay_model: `true`
  checks: code=`true`, api=`true`, tests=`true`
  runtime_wired: `true`, gate_status: `ready`
  cooperative_relay_model: found paths -> crates/chimera-relay
- emergency_oob_carriers: `true`
  checks: code=`true`, api=`true`, tests=`true`
  runtime_wired: `true`, gate_status: `ready`
  emergency_oob_carriers: found paths -> crates/chimera-emergency
- roaming_cache: `true`
  checks: code=`true`, api=`true`, tests=`true`
  runtime_wired: `true`, gate_status: `ready`
  roaming_cache: found paths -> crates/chimera-roaming
- reputation_complaint_credit: `true`
  checks: code=`true`, api=`true`, tests=`true`
  runtime_wired: `true`, gate_status: `ready`
  reputation_complaint_credit: found paths -> crates/chimera-reputation
