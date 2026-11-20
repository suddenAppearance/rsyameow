//! Unit tests for queue and wave helpers.

use super::RootView;
use super::playback::step_index;
use yandex_music::Track;

fn track(id: &str) -> Track {
    Track {
        id: id.to_string(),
        title: None,
        version: None,
        available: None,
        artists: Vec::new(),
        albums: Vec::new(),
        cover_uri: None,
        duration_ms: None,
        explicit: None,
        available_for_premium_users: None,
        preview_duration_ms: None,
        short_description: None,
        track_type: None,
    }
}

#[test]
fn queue_index_wraps_in_both_directions() {
    assert_eq!(step_index(None, 3, true), 0);
    assert_eq!(step_index(Some(0), 3, true), 1);
    assert_eq!(step_index(Some(2), 3, true), 0);
    assert_eq!(step_index(Some(0), 3, false), 2);
    assert_eq!(step_index(Some(2), 3, false), 1);
    assert_eq!(step_index(Some(5), 3, true), 0);
    assert_eq!(step_index(Some(0), 0, true), 0);
}

#[test]
fn wave_batches_never_add_known_tracks() {
    let existing = vec![track("1")];
    let queue = vec!["2".to_string(), "3".to_string()];
    let incoming = vec![track("1"), track("2"), track("3"), track("4")];
    let fresh = RootView::fresh_wave_tracks(incoming, &existing, Some(&queue));
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].id, "4");
}

#[test]
fn initial_wave_batch_drops_internal_duplicates() {
    let incoming = vec![track("1"), track("1"), track("2")];
    let fresh = RootView::fresh_wave_tracks(incoming, &[], None);
    assert_eq!(
        fresh
            .iter()
            .map(|track| track.id.as_str())
            .collect::<Vec<_>>(),
        vec!["1", "2"]
    );
}
