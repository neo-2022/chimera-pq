use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use chimera_mesh::{RouteAnnouncement, format_route_announcements, parse_route_announcements};

use super::options::Options;

pub type SharedRouteAnnouncementRegistry = Arc<Mutex<Vec<RouteAnnouncement>>>;

pub fn new_shared_route_announcement_registry() -> SharedRouteAnnouncementRegistry {
    Arc::new(Mutex::new(Vec::new()))
}

pub fn local_announcements_from_options(options: &Options) -> Vec<RouteAnnouncement> {
    parse_mesh_announcements_value(&options.mesh_policy_payload)
}

fn parse_mesh_announcements_value(payload: &str) -> Vec<RouteAnnouncement> {
    for segment in payload.split(';') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let Some((key, value)) = segment.split_once('=') else {
            continue;
        };
        if key.trim().eq_ignore_ascii_case("mesh_announcements") {
            return parse_route_announcements(value.trim()).unwrap_or_default();
        }
    }
    Vec::new()
}

pub fn merge_received_announcements(
    registry: &SharedRouteAnnouncementRegistry,
    announcements: &[RouteAnnouncement],
) -> Result<bool, String> {
    if announcements.is_empty() {
        return Ok(false);
    }
    let mut guard = registry
        .lock()
        .map_err(|_| "route announcement registry lock poisoned".to_string())?;
    let now = SystemTime::now();
    let mut keys: BTreeSet<String> = guard.iter().map(announcement_key).collect();
    let mut added = 0usize;
    for announcement in announcements {
        if announcement.is_expired(now) {
            continue;
        }
        let key = announcement_key(announcement);
        if keys.insert(key) {
            guard.push(announcement.clone());
            added = added.saturating_add(1);
        }
    }
    Ok(added > 0)
}

pub fn registry_announcements(registry: &SharedRouteAnnouncementRegistry) -> Vec<RouteAnnouncement> {
    registry
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

pub fn format_registry_for_log(registry: &SharedRouteAnnouncementRegistry) -> String {
    let guard = match registry.lock() {
        Ok(guard) => guard,
        Err(_) => return "registry_poisoned".to_string(),
    };
    let count = guard.len();
    let payload = format_route_announcements(&guard);
    if payload.len() > 256 {
        format!("count={count};value=<redacted_len={}>", payload.len())
    } else {
        format!("count={count};value={payload}")
    }
}

fn announcement_key(announcement: &RouteAnnouncement) -> String {
    format!(
        "{}|{}|{}",
        announcement.destination().to_wire_string(),
        announcement.via().as_str(),
        announcement.route_binding_id().get()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_announcements() -> Vec<RouteAnnouncement> {
        parse_route_announcements(
            "static,cidr/192.168.31.0/24,vdsina,3600,7|static,domain/example.internal,amai,1800,11",
        )
        .expect("sample announcements must parse")
    }

    #[test]
    fn registry_deduplicates_by_destination_via_binding() {
        let registry = new_shared_route_announcement_registry();
        let first = sample_announcements();
        let mut second = first.clone();
        second.push(
            parse_route_announcements("static,cidr/10.0.0.0/8,vdsina,3600,9")
                .expect("parse")
                .pop()
                .unwrap(),
        );

        assert!(
            merge_received_announcements(&registry, &first).expect("merge first"),
            "first merge should change registry"
        );
        assert!(
            merge_received_announcements(&registry, &second).expect("merge second"),
            "second merge should add the new announcement"
        );

        let all = registry_announcements(&registry);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn registry_drops_expired_announcements() {
        let registry = new_shared_route_announcement_registry();
        let parsed = parse_route_announcements("static,cidr/192.168.31.0/24,vdsina,1,7")
            .expect("parse");
        std::thread::sleep(std::time::Duration::from_millis(1200));
        assert!(
            !merge_received_announcements(&registry, &parsed).expect("merge"),
            "expired announcements should not change registry"
        );
    }
}
