//! Reading kstatus conditions: when an object went wrong.

use chrono::{DateTime, Utc};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;

/// When the object's failure markers last changed: the newest transition time
/// among `Stalled=True` and `Ready=False` — what `io::set_ready` writes at the
/// failing transition. `None` while neither holds; never a last-success time,
/// which a later failure leaves unchanged and would make a new failure look old.
pub fn failing_since(conditions: &[Condition]) -> Option<DateTime<Utc>> {
    conditions
        .iter()
        .filter(|c| {
            (c.type_ == kopiur_api::consts::STALLED_CONDITION && c.status == "True")
                || (c.type_ == kopiur_api::consts::READY_CONDITION && c.status == "False")
        })
        .filter_map(|c| DateTime::from_timestamp(c.last_transition_time.0.as_second(), 0))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;

    fn condition(type_: &str, status: &str, at: &str) -> Condition {
        Condition {
            type_: type_.into(),
            status: status.into(),
            reason: "Reason".into(),
            message: String::new(),
            last_transition_time: Time(at.parse().unwrap()),
            observed_generation: None,
        }
    }

    #[test]
    fn the_newest_failure_marker_dates_it() {
        let at = failing_since(&[
            condition("Ready", "False", "2026-06-11T12:00:00Z"),
            condition("Stalled", "True", "2026-06-11T13:00:00Z"),
        ]);
        assert_eq!(at.unwrap().to_rfc3339(), "2026-06-11T13:00:00+00:00");
    }

    #[test]
    fn a_healthy_object_is_undated() {
        assert_eq!(
            failing_since(&[
                condition("Ready", "True", "2026-06-11T12:00:00Z"),
                condition("Stalled", "False", "2026-06-11T13:00:00Z"),
                condition("Reconciling", "True", "2026-06-11T14:00:00Z"),
            ]),
            None
        );
        assert_eq!(failing_since(&[]), None);
    }
}
