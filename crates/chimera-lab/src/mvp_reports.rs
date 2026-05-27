use crate::{Language, MvpSpecCheckResult};

pub(crate) fn render_mvp_spec_check_json(result: MvpSpecCheckResult) -> String {
    format!(
        "{{\"status\":\"ok\",\"kind\":\"mvp_spec_check\",\"m0_workspace\":{},\"m1_local_tunnel\":{},\"m2_crypto_session\":{},\"m3_carrier_validation\":{},\"m4_routing_determinism\":{},\"m5_doctor_and_config\":{},\"m6_hardening\":{},\"network_state\":\"not_modified\"}}",
        result.m0_workspace,
        result.m1_local_tunnel,
        result.m2_crypto_session,
        result.m3_carrier_validation,
        result.m4_routing_determinism,
        result.m5_doctor_and_config,
        result.m6_hardening
    )
}

pub(crate) fn render_mvp_spec_report_markdown(result: MvpSpecCheckResult) -> String {
    let all_ok = result.m0_workspace
        && result.m1_local_tunnel
        && result.m2_crypto_session
        && result.m3_carrier_validation
        && result.m4_routing_determinism
        && result.m5_doctor_and_config
        && result.m6_hardening;
    format!(
        "# CHIMERA-PQ MVP Spec Coverage\n\n\
Status: **{}**\n\n\
Checklist:\n\
- M0 workspace/tooling: `{}`\n\
- M1 local tunnel: `{}`\n\
- M2 crypto/session: `{}`\n\
- M3 carrier validation: `{}`\n\
- M4 routing determinism: `{}`\n\
- M5 doctor/config: `{}`\n\
- M6 hardening: `{}`\n\n\
Network safety: no OS route/DNS/firewall/proxy changes in this report path.\n",
        if all_ok { "PASS" } else { "FAIL" },
        result.m0_workspace,
        result.m1_local_tunnel,
        result.m2_crypto_session,
        result.m3_carrier_validation,
        result.m4_routing_determinism,
        result.m5_doctor_and_config,
        result.m6_hardening
    )
}

pub(crate) fn render_m5_artifacts_report_markdown(
    lang: Language,
    all_ok: bool,
    config_ok: bool,
    doctor_ok: bool,
    route_ok: bool,
    datapath_ok: bool,
    rollback_ok: bool,
) -> String {
    match lang {
        Language::En => format!(
            "# M5 Artifacts Report\n\n\
Status: **{}**\n\n\
Checks:\n\
- Config smoke: `{}`\n\
- Doctor artifacts: `{}`\n\
- Route explain artifact: `{}`\n\
- Datapath artifact: `{}`\n\
- Rollback artifacts: `{}`\n\n\
Artifacts:\n\
- `docs/doctor_latest.json`\n\
- `docs/gateway_doctor_latest.json`\n\
- `docs/lab_doctor_latest.json`\n\
- `docs/datapath_latest.json`\n\
- `docs/route_explain_latest.json`\n\
- `docs/rollback_status_latest.json`\n\
- `docs/rollback_recover_latest.json`\n\
- `docs/rollback_status_after_recover_latest.json`\n\n\
Network safety: no OS route/DNS/firewall/proxy changes in this report path.\n",
            if all_ok { "PASS" } else { "FAIL" },
            config_ok,
            doctor_ok,
            route_ok,
            datapath_ok,
            rollback_ok
        ),
        Language::Ru => format!(
            "# Отчет По Артефактам M5\n\n\
Статус: **{}**\n\n\
Проверки:\n\
- Config smoke: `{}`\n\
- Артефакты doctor: `{}`\n\
- Артефакт route explain: `{}`\n\
- Артефакт datapath: `{}`\n\
- Артефакты rollback: `{}`\n\n\
Артефакты:\n\
- `docs/doctor_latest.json`\n\
- `docs/gateway_doctor_latest.json`\n\
- `docs/lab_doctor_latest.json`\n\
- `docs/datapath_latest.json`\n\
- `docs/route_explain_latest.json`\n\
- `docs/rollback_status_latest.json`\n\
- `docs/rollback_recover_latest.json`\n\
- `docs/rollback_status_after_recover_latest.json`\n\n\
Безопасность сети: в этом отчете мы не меняем маршруты/DNS/firewall/proxy ОС.\n",
            if all_ok { "PASS" } else { "FAIL" },
            config_ok,
            doctor_ok,
            route_ok,
            datapath_ok,
            rollback_ok
        ),
    }
}

pub(crate) fn render_m6_artifacts_report_markdown(
    lang: Language,
    hardening_ok: bool,
    benchmark_ok: bool,
    mvp_check_ok: bool,
) -> String {
    match lang {
        Language::En => format!(
            "# M6 Artifacts Report\n\n\
Status: **{}**\n\n\
Checks:\n\
- Benchmark artifact: `{}`\n\
- MVP spec check artifact (M6): `{}`\n\n\
Artifacts:\n\
- `docs/benchmark_latest.json`\n\
- `docs/mvp_spec_check_latest.json`\n\n\
Network safety: no OS route/DNS/firewall/proxy changes in this report path.\n",
            if hardening_ok { "PASS" } else { "FAIL" },
            benchmark_ok,
            mvp_check_ok
        ),
        Language::Ru => format!(
            "# Отчет По Артефактам M6\n\n\
Статус: **{}**\n\n\
Проверки:\n\
- Артефакт benchmark: `{}`\n\
- Артефакт mvp-spec-check (M6): `{}`\n\n\
Артефакты:\n\
- `docs/benchmark_latest.json`\n\
- `docs/mvp_spec_check_latest.json`\n\n\
Безопасность сети: в этом отчете мы не меняем маршруты/DNS/firewall/proxy ОС.\n",
            if hardening_ok { "PASS" } else { "FAIL" },
            benchmark_ok,
            mvp_check_ok
        ),
    }
}
