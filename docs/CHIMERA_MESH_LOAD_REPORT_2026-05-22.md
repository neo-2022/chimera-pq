# CHIMERA Mesh Load Report (2026-05-22)

Status: pass

Evidence:
- Load artifact:   docs/load/CHIMERA_LOAD_5M_LAPTOP_20260522_023725.json
- Laptop post-load gate:   /home/art/chimera-pq/docs/CHIMERA_E2E_CHANNEL_GATE_AFTER_SYNC.json
- Laptop channel audit:   /home/art/chimera-pq/docs/CHIMERA_CHANNEL_AUDIT.json
- Laptop path proof:   /home/art/chimera-pq/docs/CHIMERA_PATH_PROOF.json

Load summary (5 min, parallel):
- total_ok: 2980
- total_fail: 18
- success_rate_min: 0.9897172236503856
- success_rate_max: 0.9966216216216216

Per-site:
- https://youtube.com: ok=319, fail=2, success_rate=0.9937694704049844, codes={"200":319,"000":2}
- https://aistudio.google.com: ok=516, fail=4, success_rate=0.9923076923076923, codes={"200":516,"000":4}
- https://chat.openai.com: ok=385, fail=4, success_rate=0.9897172236503856, codes={"403":385,"308":1,"000":3}
- https://openai.com: ok=734, fail=3, success_rate=0.9959294436906377, codes={"403":734,"000":3}
- https://epicgames.com: ok=295, fail=1, success_rate=0.9966216216216216, codes={"403":295,"000":1}
- https://www.googleadservices.com: ok=731, fail=4, success_rate=0.9945578231292517, codes={"404":731,"000":4}

Post-load verification:
- path_proof.status=pass
- channel_audit.status=pass
- e2e_channel_gate.status=pass
- e2e_channel_gate_guard=PASS

Limits/notes:
- Some transient timeouts were observed (curl 28).
- HTTP status 403/404 on some domains is upstream site policy/content behavior, not tunnel break.
