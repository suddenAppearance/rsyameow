//! A small command-line smoke test for the Yandex Music client.
//!
//! ```sh
//! YANDEX_MUSIC_TOKEN=... cargo run -p yandex-music --example smoke
//! ```

use std::env;
use std::time::Duration;
use yandex_music::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token =
        env::var("YANDEX_MUSIC_TOKEN").map_err(|_| "set YANDEX_MUSIC_TOKEN to an OAuth token")?;
    let client = Client::with_token(token);

    let status = client.init()?;
    let account = status.account.as_ref();
    println!(
        "account: {} (uid {:?})",
        account
            .map(|account| account.best_name())
            .unwrap_or("unknown"),
        account.and_then(|account| account.uid)
    );
    println!(
        "plus: {}, permissions: {:?}",
        status.has_plus(),
        status
            .permissions
            .as_ref()
            .map(|permissions| &permissions.values)
    );

    let liked = client.liked_tracks()?;
    println!("liked tracks: {}", liked.tracks.len());

    if let Some(short) = liked.tracks.first() {
        let track = client.track(&short.track_id())?;
        println!(
            "first track: {} — {} ({:?})",
            track.artist_names(),
            track.display_title(),
            track.duration()
        );

        let info = client.download_info(&track.id)?;
        for variant in &info {
            println!(
                "  variant: {} {} kbps preview={}",
                variant.codec, variant.bitrate_in_kbps, variant.preview
            );
        }

        let downloaded = client.download_track(&track.id)?;
        let path = env::temp_dir().join(format!("rsyameow-{}.mp3", track.id));
        std::fs::write(&path, downloaded.bytes)?;
        println!(
            "downloaded {} bytes ({} kbps) to {}",
            std::fs::metadata(&path)?.len(),
            downloaded.bitrate_kbps,
            path.display()
        );

        // Give the example a chance to prove the URL is still valid briefly.
        std::thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
