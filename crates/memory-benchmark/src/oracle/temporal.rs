use crate::Event;

pub fn visible_at<'a>(
    events: &'a [Event],
    world_time: Option<&str>,
    tx_time: Option<&str>,
    review_time: Option<&str>,
    policy_time: Option<&str>,
) -> Vec<&'a Event> {
    events
        .iter()
        .filter(|event| {
            if let Some(tx) = tx_time {
                if event.tx_time.as_str() > tx {
                    return false;
                }
            }
            if let Some(world) = world_time {
                if let Some(valid_from) = event.valid_from.as_deref() {
                    if valid_from > world {
                        return false;
                    }
                }
                if let Some(valid_to) = event.valid_to.as_deref() {
                    if valid_to <= world {
                        return false;
                    }
                }
            }
            if let (Some(required), Some(actual)) = (review_time, event.review_time.as_deref()) {
                if actual > required {
                    return false;
                }
            }
            if let (Some(required), Some(actual)) = (policy_time, event.policy_time.as_deref()) {
                if actual > required {
                    return false;
                }
            }
            true
        })
        .collect()
}
