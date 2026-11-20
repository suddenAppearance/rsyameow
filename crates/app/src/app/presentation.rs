//! Formatting helpers shared by the profile UI.

use super::*;

pub(crate) fn registered_year(status: &Status) -> String {
    status
        .account
        .as_ref()
        .and_then(|account| account.registered_at.as_deref())
        .and_then(|date| date.get(..4))
        .unwrap_or("—")
        .to_string()
}

/// Human-readable plan name.
pub(crate) fn plan_name(status: &Status) -> String {
    if status.has_plus() {
        "Yandex Plus".into()
    } else {
        "Без подписки".into()
    }
}

/// Subscription end date for the profile.
pub(crate) fn plus_until(status: &Status) -> String {
    status
        .subscription
        .as_ref()
        .and_then(|subscription| subscription.end.as_deref())
        .map(format::date)
        .or_else(|| {
            status
                .permissions
                .as_ref()
                .and_then(|permissions| permissions.until.as_deref())
                .map(format::date)
        })
        .unwrap_or_else(|| "—".into())
}
