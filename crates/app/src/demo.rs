//! Demo data used by `--demo` to preview the UI without a Yandex account.

use gpui::{Image, Rgba};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use yandex_music::{
    Account, Artist, Permissions, Playlist, Plus, Status, Subscription, Track, TrackShort, User,
};

/// Fictional listeners and tracks. They are intentionally not tied to real
/// artists so that the demo cannot be mistaken for live Yandex data.
pub fn status() -> Status {
    Status {
        account: Some(Account {
            now: Some("2026-11-20T00:00:00".into()),
            uid: Some(100042),
            login: Some("demo@ya.myau".into()),
            full_name: Some("Демид Кот".into()),
            first_name: Some("Демид".into()),
            display_name: Some("Демид Кот".into()),
            service_available: Some(true),
            region: Some(213),
            region_code: Some("ru".into()),
            registered_at: Some("2023-02-14T10:00:00".into()),
            ..Default::default()
        }),
        permissions: Some(Permissions {
            until: Some("2027-02-14T00:00:00".into()),
            values: vec!["plus".into()],
            default: vec![],
        }),
        plus: Some(Plus {
            has_plus: true,
            is_tutorial_completed: true,
            migrated: None,
        }),
        subscription: Some(Subscription {
            end: Some("2027-02-14T00:00:00".into()),
            can_start_trial: Some(false),
            ..Default::default()
        }),
        default_email: Some("demo@ya.myau".into()),
        station_exists: Some(true),
        pretrial_active: Some(false),
        ..Default::default()
    }
}

fn track(id: &str, title: &str, artist: &str, album: &str, duration_ms: u64, year: u64) -> Track {
    Track {
        id: id.into(),
        title: Some(title.into()),
        available: Some(true),
        artists: vec![Artist {
            id: Some(1000 + id.parse::<u64>().unwrap_or(1)),
            name: Some(artist.into()),
            likes_count: Some(120_000 + id.parse::<u64>().unwrap_or(1) * 137),
            ..Default::default()
        }],
        albums: vec![yandex_music::Album {
            id: Some(9000 + id.parse::<u64>().unwrap_or(1)),
            title: Some(album.into()),
            year: Some(year),
            ..Default::default()
        }],
        duration_ms: Some(duration_ms),
        available_for_premium_users: Some(false),
        ..Default::default()
    }
}

/// A small, believable liked-tracks library.
pub fn tracks() -> Vec<Track> {
    let mut tracks = vec![
        track(
            "1",
            "Ночной трамвай",
            "Мяу Orchestra",
            "Окраины",
            222_000,
            2024,
        ),
        track(
            "2",
            "Северный ветер",
            "Полярный кот",
            "Белые ночи",
            245_000,
            2023,
        ),
        track(
            "3",
            "Пыль на пластинке",
            "Винтаж Кот",
            "Виниловые сны",
            178_000,
            2022,
        ),
        track(
            "4",
            "Море в наушниках",
            "Соль и Мяу",
            "Глубина",
            201_000,
            2025,
        ),
        track(
            "5",
            "Последний сеанс",
            "Кинотеатр Мур",
            "Красные кресла",
            273_000,
            2021,
        ),
        track(
            "6",
            "Тихий дождь",
            "Дом у пруда",
            "Окна настежь",
            187_000,
            2020,
        ),
        track(
            "7",
            "Полночь на кухне",
            "Лапки и джаз",
            "После ужина",
            312_000,
            2024,
        ),
    ];
    tracks[0].explicit = Some(true);
    tracks[2].explicit = Some(true);
    tracks
}

/// Fictional lyrics used by the demo track page.
pub fn demo_lyrics() -> &'static str {
    "Ночной трамвай плывёт по краю,\nГде фонари рисуют медь.\nЯ эту песню напеваю,\nЧтобы до утра успеть.\n\nПровода гудят над крышей,\nГород прячет этажи.\nТише, тише, тише, тише —\nТрамвай, до утра бежим.\n\nПусть стрелки спорят с циферблатом,\nПусть спят троллейбусы в депо.\nМы едем медленно и рядом,\nПока нам не рассветало.\n\nИ если путь закончится у парка,\nГде фонари качают тень,\nЯ попрошу у ночи только часа,\nЧтобы продлить ещё наш день."
}

fn make_playlist(
    kind: u64,
    title: &str,
    description: &str,
    visibility: &str,
    tracks: &[Track],
) -> Playlist {
    let duration_ms = tracks
        .iter()
        .map(|track| track.duration_ms.unwrap_or(0))
        .sum::<u64>();

    Playlist {
        uid: Some(100042),
        kind: Some(kind),
        title: Some(title.into()),
        track_count: Some(tracks.len() as u64),
        visibility: Some(visibility.into()),
        description: Some(description.into()),
        cover: None,
        og_image: None,
        owner: Some(User {
            uid: Some(100042),
            login: Some("demo@ya.myau".into()),
            display_name: Some("Демид Кот".into()),
            full_name: Some("Демид Кот".into()),
        }),
        tracks: liked(tracks),
        duration_ms: Some(duration_ms),
        generated_playlist_type: None,
        created: None,
        modified: None,
        snapshot: Some(1),
        playlist_uuid: Some(format!("demo-playlist-{kind}")),
    }
}

/// Small fictional playlist library for the demo account.
pub fn playlists(tracks: &[Track]) -> Vec<Playlist> {
    vec![
        make_playlist(
            101,
            "Ночная поездка",
            "Трамваи, фонари и немного меланхолии.",
            "public",
            &[tracks[0].clone(), tracks[1].clone(), tracks[4].clone()],
        ),
        make_playlist(
            102,
            "Утро на кухне",
            "Кофе, окно и медленные треки.",
            "public",
            &[tracks[3].clone(), tracks[5].clone(), tracks[6].clone()],
        ),
        make_playlist(
            103,
            "Всё подряд",
            "Личная коллекция демо-кота.",
            "private",
            tracks,
        ),
    ]
}

pub fn liked(tracks: &[Track]) -> Vec<TrackShort> {
    tracks
        .iter()
        .map(|track| TrackShort {
            id: track.id.clone(),
            album_id: track
                .albums
                .first()
                .and_then(|album| album.id)
                .map(|id| id.to_string()),
            timestamp: None,
            track: None,
            ..Default::default()
        })
        .collect()
}

/// Generate abstract gradient artwork for every demo track.
/// A gradient palette matching the generated demo cover for the given index.
pub fn cover_palette(index: usize) -> (Rgba, Rgba) {
    let palettes: [(u32, u32); 7] = [
        (0x2A1B3D, 0xF7B267),
        (0x0B3D2E, 0x9AD1D4),
        (0x3D0C11, 0xF2C078),
        (0x102A43, 0xFFDB4D),
        (0x2B2D42, 0xEF8354),
        (0x1C2541, 0x5BC0BE),
        (0x3A0CA3, 0xF72585),
    ];
    let (start, end) = palettes[index % palettes.len()];
    let start = rgb_components(start);
    let end = rgb_components(end);
    (
        Rgba {
            r: start[0] / 255.0 * 0.38,
            g: start[1] / 255.0 * 0.38,
            b: start[2] / 255.0 * 0.38,
            a: 1.0,
        },
        Rgba {
            r: end[0] / 255.0,
            g: end[1] / 255.0,
            b: end[2] / 255.0,
            a: 1.0,
        },
    )
}

pub fn covers(tracks: &[Track]) -> HashMap<String, Arc<Image>> {
    let palettes: [(u32, u32); 7] = [
        (0x2A1B3D, 0xF7B267),
        (0x0B3D2E, 0x9AD1D4),
        (0x3D0C11, 0xF2C078),
        (0x102A43, 0xFFDB4D),
        (0x2B2D42, 0xEF8354),
        (0x1C2541, 0x5BC0BE),
        (0x3A0CA3, 0xF72585),
    ];

    tracks
        .iter()
        .enumerate()
        .filter_map(|(index, track)| {
            let (start, end) = palettes[index % palettes.len()];
            let png = render_cover(start, end, index as u32)?;
            Some((
                track.id.clone(),
                Arc::new(Image::from_bytes(gpui::ImageFormat::Png, png)),
            ))
        })
        .collect()
}

fn render_cover(start: u32, end: u32, seed: u32) -> Option<Vec<u8>> {
    const SIZE: u32 = 360;
    let mut image = image::RgbaImage::new(SIZE, SIZE);
    let start = rgb_components(start);
    let end = rgb_components(end);

    let center_x = 40.0 + (seed as f32 * 53.0) % 260.0;
    let center_y = 80.0 + (seed as f32 * 37.0) % 220.0;
    let radius = 70.0 + (seed as f32 * 11.0) % 70.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let t = ((x + y) as f32) / ((SIZE + SIZE) as f32);
            let mut color = [
                lerp(start[0], end[0], t),
                lerp(start[1], end[1], t),
                lerp(start[2], end[2], t),
            ];

            // Vignette.
            let dx = x as f32 / SIZE as f32 - 0.5;
            let dy = y as f32 / SIZE as f32 - 0.5;
            let vignette = (1.0 - (dx * dx + dy * dy) * 1.35).clamp(0.35, 1.0);
            for channel in &mut color {
                *channel *= vignette;
            }

            // Translucent orb.
            let distance = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
            if distance < radius {
                let alpha = (1.0 - distance / radius).powf(1.7) * 0.55;
                for (channel, warm) in color.iter_mut().zip([255.0, 219.0, 77.0]) {
                    *channel = *channel * (1.0 - alpha) + warm * alpha;
                }
            }

            // Thin diagonal accent line.
            let line = (x as f32 * 0.7 + y as f32 * 0.7 - 250.0).abs();
            if line < 1.6 {
                for channel in &mut color {
                    *channel = (*channel * 0.6 + 255.0 * 0.4).min(255.0);
                }
            }

            image.put_pixel(
                x,
                y,
                image::Rgba([
                    color[0].clamp(0.0, 255.0) as u8,
                    color[1].clamp(0.0, 255.0) as u8,
                    color[2].clamp(0.0, 255.0) as u8,
                    255,
                ]),
            );
        }
    }

    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut png, image::ImageFormat::Png)
        .ok()?;
    Some(png.into_inner())
}

fn rgb_components(value: u32) -> [f32; 3] {
    [
        ((value >> 16) & 0xff) as f32,
        ((value >> 8) & 0xff) as f32,
        (value & 0xff) as f32,
    ]
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}
