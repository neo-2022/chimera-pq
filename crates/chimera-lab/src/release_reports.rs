use crate::{Language, MvpSpecCheckResult, ReleaseGateChecklist, ReleaseReadinessArtifacts};

pub(crate) fn render_release_readiness_report_markdown(
    lang: Language,
    release_ok: bool,
    real_world_datapath_closed: bool,
    result: MvpSpecCheckResult,
    checklist: ReleaseGateChecklist,
    artifacts: ReleaseReadinessArtifacts,
) -> String {
    match lang {
        Language::En => format!(
            "# Release Readiness Report\n\n\
Status: **{}**\n\n\
Simple meaning: if status is PASS, MVP is ready for wider lab validation only (not a real-world datapath closure claim).\n\n\
Release gate (spec section 11):\n\
- Clean clone builds: `{}`\n\
- Client and gateway run on Linux: `{}`\n\
- Encrypted tunnel carries traffic: `{}`\n\
- Policy routing works (direct/gateway/block): `{}`\n\
- DNS binding works: `{}`\n\
- Route explain works: `{}`\n\
- Shutdown restores network state: `{}`\n\
- Security tests pass: `{}`\n\
- Parser fuzz smoke passes: `{}`\n\
- No raw secrets/tokens in logs: `{}`\n\
- Benchmark report exists: `{}`\n\
- Operations guide exists: `{}`\n\
- Runtime DNS apply verified: `{}`\n\
- Runtime route apply verified: `{}`\n\
- Runtime route-policy validation verified: `{}`\n\
- Runtime TUN-name validation verified: `{}`\n\
- Runtime forced-stop rollback verified: `{}`\n\n\
Milestones:\n\
- M0 workspace/tooling: `{}`\n\
- M1 local tunnel: `{}`\n\
- M2 crypto/session: `{}`\n\
- M3 carrier validation: `{}`\n\
- M4 routing determinism: `{}`\n\
- M5 practical diagnostics: `{}`\n\
- M6 hardening: `{}`\n\n\
Artifacts:\n\
- M5 artifacts report: `{}` (`docs/M5_ARTIFACTS_REPORT.md`)\n\
- M6 artifacts report: `{}` (`docs/M6_ARTIFACTS_REPORT.md`)\n\
- Benchmark artifact: `{}` (`docs/benchmark_latest.json`)\n\
- CEF phase1 smoke: `{}` (`docs/CEF_PHASE1_SMOKE.json`)\n\
- Mesh route explain: `{}` (`docs/MESH_ROUTE_EXPLAIN.json`)\n\
- Mesh auto adaptive trace: `{}` (`docs/MESH_AUTO_ADAPTIVE_TRACE.json`)\n\n\
Truth boundary:\n\
- Lab/proof/report contour only: `true`\n\
- Real OS-level datapath closure (strict M4/M5): `{}`\n\n\
Network safety: no OS route/DNS/firewall/proxy changes in this report path.\n",
            if release_ok { "PASS" } else { "FAIL" },
            checklist.clean_clone_builds,
            checklist.client_gateway_run_linux,
            checklist.encrypted_tunnel_carries_traffic,
            checklist.policy_routing_direct_gateway_block,
            checklist.dns_binding_works,
            checklist.route_explain_works,
            checklist.shutdown_restores_network_state,
            checklist.security_tests_pass,
            checklist.parser_fuzz_smoke_passes,
            checklist.no_raw_secrets_in_logs,
            checklist.benchmark_report_exists,
            checklist.operations_guide_exists,
            checklist.runtime_apply_dns_verified,
            checklist.runtime_apply_route_verified,
            checklist.runtime_route_policy_validation_verified,
            checklist.runtime_tun_name_validation_verified,
            checklist.runtime_forced_stop_rollback_verified,
            result.m0_workspace,
            result.m1_local_tunnel,
            result.m2_crypto_session,
            result.m3_carrier_validation,
            result.m4_routing_determinism,
            result.m5_doctor_and_config,
            result.m6_hardening,
            artifacts.m5_report_ok,
            artifacts.m6_report_ok,
            artifacts.benchmark_ok,
            artifacts.cef_phase1_smoke_ok,
            artifacts.mesh_route_explain_ok,
            artifacts.mesh_auto_adaptive_ok,
            real_world_datapath_closed
        ),
        Language::Ru => format!(
            "# Отчет Готовности Релиза\n\n\
Статус: **{}**\n\n\
Просто: если статус PASS, MVP готов только к расширенным лабораторным тестам (это не означает закрытие real-world datapath).\n\n\
Release gate (раздел 11 спеки):\n\
- Чистая копия репозитория собирается: `{}`\n\
- Клиент и gateway запускаются на Linux: `{}`\n\
- Зашифрованный tunnel передает трафик: `{}`\n\
- Policy routing работает (direct/gateway/block): `{}`\n\
- DNS binding работает: `{}`\n\
- Route explain работает: `{}`\n\
- Shutdown восстанавливает состояние сети: `{}`\n\
- Security-тесты проходят: `{}`\n\
- Fuzz smoke для parser проходит: `{}`\n\
- В логах нет сырых секретов/токенов: `{}`\n\
- Benchmark-отчет существует: `{}`\n\
- Operations guide существует: `{}`\n\
- Runtime DNS apply подтвержден: `{}`\n\
- Runtime route apply подтвержден: `{}`\n\
- Runtime route-policy validation подтвержден: `{}`\n\
- Runtime TUN-name validation подтвержден: `{}`\n\
- Runtime rollback после forced-stop подтвержден: `{}`\n\n\
Этапы:\n\
- M0 workspace/tooling: `{}`\n\
- M1 локальный tunnel: `{}`\n\
- M2 crypto/session: `{}`\n\
- M3 валидация carrier: `{}`\n\
- M4 детерминизм маршрутизации: `{}`\n\
- M5 практическая диагностика: `{}`\n\
- M6 hardening: `{}`\n\n\
Артефакты:\n\
- Отчет артефактов M5: `{}` (`docs/M5_ARTIFACTS_REPORT.md`)\n\
- Отчет артефактов M6: `{}` (`docs/M6_ARTIFACTS_REPORT.md`)\n\
- Артефакт benchmark: `{}` (`docs/benchmark_latest.json`)\n\
- CEF phase1 smoke: `{}` (`docs/CEF_PHASE1_SMOKE.json`)\n\
- Mesh route explain: `{}` (`docs/MESH_ROUTE_EXPLAIN.json`)\n\
- Mesh auto adaptive trace: `{}` (`docs/MESH_AUTO_ADAPTIVE_TRACE.json`)\n\n\
Граница истины:\n\
- Контур lab/proof/report: `true`\n\
- Real OS-level datapath closure (strict M4/M5): `{}`\n\n\
Безопасность сети: в этом отчете мы не меняем маршруты/DNS/firewall/proxy ОС.\n",
            if release_ok { "PASS" } else { "FAIL" },
            checklist.clean_clone_builds,
            checklist.client_gateway_run_linux,
            checklist.encrypted_tunnel_carries_traffic,
            checklist.policy_routing_direct_gateway_block,
            checklist.dns_binding_works,
            checklist.route_explain_works,
            checklist.shutdown_restores_network_state,
            checklist.security_tests_pass,
            checklist.parser_fuzz_smoke_passes,
            checklist.no_raw_secrets_in_logs,
            checklist.benchmark_report_exists,
            checklist.operations_guide_exists,
            checklist.runtime_apply_dns_verified,
            checklist.runtime_apply_route_verified,
            checklist.runtime_route_policy_validation_verified,
            checklist.runtime_tun_name_validation_verified,
            checklist.runtime_forced_stop_rollback_verified,
            result.m0_workspace,
            result.m1_local_tunnel,
            result.m2_crypto_session,
            result.m3_carrier_validation,
            result.m4_routing_determinism,
            result.m5_doctor_and_config,
            result.m6_hardening,
            artifacts.m5_report_ok,
            artifacts.m6_report_ok,
            artifacts.benchmark_ok,
            artifacts.cef_phase1_smoke_ok,
            artifacts.mesh_route_explain_ok,
            artifacts.mesh_auto_adaptive_ok,
            real_world_datapath_closed
        ),
    }
}

pub(crate) fn render_release_readiness_report_json(
    release_ok: bool,
    real_world_datapath_closed: bool,
    result: MvpSpecCheckResult,
    checklist: ReleaseGateChecklist,
    artifacts: ReleaseReadinessArtifacts,
) -> String {
    format!(
        "{{\"status\":\"{}\",\"kind\":\"release_readiness_report\",\"message_en\":\"Release readiness check finished.\",\"message_ru\":\"Проверка готовности релиза завершена.\",\"release_ok\":{},\"truth_boundary\":{{\"lab_scope_only\":true,\"real_world_datapath_closed\":{}}},\"milestones\":{{\"m0_workspace\":{},\"m1_local_tunnel\":{},\"m2_crypto_session\":{},\"m3_carrier_validation\":{},\"m4_routing_determinism\":{},\"m5_doctor_and_config\":{},\"m6_hardening\":{}}},\"release_gate\":{{\"clean_clone_builds\":{},\"client_gateway_run_linux\":{},\"encrypted_tunnel_carries_traffic\":{},\"policy_routing_direct_gateway_block\":{},\"dns_binding_works\":{},\"route_explain_works\":{},\"shutdown_restores_network_state\":{},\"security_tests_pass\":{},\"parser_fuzz_smoke_passes\":{},\"no_raw_secrets_in_logs\":{},\"benchmark_report_exists\":{},\"operations_guide_exists\":{},\"runtime_apply_dns_verified\":{},\"runtime_apply_route_verified\":{},\"runtime_route_policy_validation_verified\":{},\"runtime_tun_name_validation_verified\":{},\"runtime_forced_stop_rollback_verified\":{}}},\"artifacts\":{{\"m5_report\":{},\"m6_report\":{},\"benchmark\":{},\"cef_phase1_smoke\":{},\"mesh_route_explain\":{},\"mesh_auto_adaptive_trace\":{}}},\"network_state\":\"not_modified\"}}",
        if release_ok { "ok" } else { "fail" },
        release_ok,
        real_world_datapath_closed,
        result.m0_workspace,
        result.m1_local_tunnel,
        result.m2_crypto_session,
        result.m3_carrier_validation,
        result.m4_routing_determinism,
        result.m5_doctor_and_config,
        result.m6_hardening,
        checklist.clean_clone_builds,
        checklist.client_gateway_run_linux,
        checklist.encrypted_tunnel_carries_traffic,
        checklist.policy_routing_direct_gateway_block,
        checklist.dns_binding_works,
        checklist.route_explain_works,
        checklist.shutdown_restores_network_state,
        checklist.security_tests_pass,
        checklist.parser_fuzz_smoke_passes,
        checklist.no_raw_secrets_in_logs,
        checklist.benchmark_report_exists,
        checklist.operations_guide_exists,
        checklist.runtime_apply_dns_verified,
        checklist.runtime_apply_route_verified,
        checklist.runtime_route_policy_validation_verified,
        checklist.runtime_tun_name_validation_verified,
        checklist.runtime_forced_stop_rollback_verified,
        artifacts.m5_report_ok,
        artifacts.m6_report_ok,
        artifacts.benchmark_ok,
        artifacts.cef_phase1_smoke_ok,
        artifacts.mesh_route_explain_ok,
        artifacts.mesh_auto_adaptive_ok
    )
}
