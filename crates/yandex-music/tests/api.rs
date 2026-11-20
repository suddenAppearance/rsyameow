mod support;

use serde_json::json;
use std::time::Duration;
use support::{MockResponse, TestServer};
use yandex_music::{Client, ClientBuilder, Error};

fn client_for(server: &TestServer, token: Option<&str>) -> Client {
    let mut builder = ClientBuilder::new()
        .base_url(server.base_url())
        .oauth_base_url(server.base_url())
        .timeout(Duration::from_secs(5));
    if let Some(token) = token {
        builder = builder.token(token);
    }
    builder.build()
}

#[test]
fn device_code_flow_reports_pending_then_returns_token() {
    let server = TestServer::start(|request| match request.path.as_str() {
        "/device/code" => {
            assert_eq!(request.method, "POST");
            assert_eq!(
                request.header("X-Yandex-Music-Client"),
                Some("YandexMusicAndroid/24023621")
            );
            let form = request.form();
            assert!(
                form.iter()
                    .any(|(key, value)| key == "client_id" && value == "test-client")
            );
            assert!(
                form.iter()
                    .any(|(key, value)| key == "device_name" && value == "Test Device")
            );
            let device_id = request.form_value("device_id").unwrap();
            assert_eq!(device_id.len(), 10);
            assert!(device_id.chars().all(|ch| ch.is_ascii_alphanumeric()));

            MockResponse::json(
                200,
                json!({
                    "device_code": "device-code-1",
                    "user_code": "KITTEN42",
                    "verification_url": "https://ya.example/device",
                    "expires_in": 300,
                    "interval": 1
                }),
            )
        }
        "/token" => {
            assert_eq!(request.method, "POST");
            match request.form_value("code").as_deref() {
                Some("device-code-1") => {
                    assert_eq!(
                        request.form_value("grant_type").as_deref(),
                        Some("device_code")
                    );
                    assert_eq!(
                        request.form_value("client_id").as_deref(),
                        Some("test-client")
                    );
                    assert_eq!(
                        request.form_value("client_secret").as_deref(),
                        Some("test-secret")
                    );
                    MockResponse::json(
                        200,
                        json!({
                            "access_token": "oauth-token",
                            "refresh_token": "refresh",
                            "expires_in": 31536000,
                            "token_type": "bearer"
                        }),
                    )
                }
                Some("never-confirmed") => MockResponse::json(
                    400,
                    json!({
                        "error": "authorization_pending",
                        "error_description": "User code not confirmed"
                    }),
                ),
                _ => MockResponse::json(400, json!({"error": "invalid_grant"})),
            }
        }
        _ => MockResponse::json(404, json!({"error": "not found"})),
    });

    let client = ClientBuilder::new()
        .base_url(server.base_url())
        .oauth_base_url(server.base_url())
        .client_id("test-client")
        .client_secret("test-secret")
        .device_name("Test Device")
        .build();

    let code = client.request_device_code().unwrap();
    assert_eq!(code.device_code, "device-code-1");
    assert_eq!(code.user_code, "KITTEN42");
    assert_eq!(code.interval, 1);
    assert_eq!(code.expires_in, 300);

    assert!(
        client
            .poll_device_token("never-confirmed")
            .unwrap()
            .is_none()
    );

    let token = client.poll_device_token("device-code-1").unwrap().unwrap();
    assert_eq!(token.access_token, "oauth-token");
    assert_eq!(token.token_type.as_deref(), Some("bearer"));
    // Matching the Python library: polling does not install the token.
    assert_eq!(client.token(), None);

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
}

#[test]
fn account_and_liked_tracks_are_parsed() {
    let server = TestServer::start(|request| {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/account/status") => {
                assert_eq!(request.header("Authorization"), Some("OAuth test-token"));
                MockResponse::json(
                    200,
                    json!({
                        "result": {
                            "account": {
                                "uid": 42,
                                "login": "listener@example.com",
                                "displayName": "Кот Слушатель",
                                "fullName": "Кот Слушатель",
                                "serviceAvailable": true,
                                "region": 213,
                                "registeredAt": "2018-04-01"
                            },
                            "permissions": {"until": "2027-01-01T00:00:00", "values": ["plus"], "default": []},
                            "plus": {"hasPlus": true, "isTutorialCompleted": true},
                            "subscription": {"end": "2027-01-01T00:00:00", "canStartTrial": false}
                        }
                    }),
                )
            }
            ("GET", "/users/42/likes/tracks") => {
                assert_eq!(request.query_value("if-modified-since-revision"), Some("0"));
                MockResponse::json(
                    200,
                    json!({
                        "result": {
                            "library": {
                                "uid": 42,
                                "revision": 17,
                                "tracks": [
                                    {"id": 101, "albumId": "555", "timestamp": "2026-01-01T00:00:00"},
                                    {"id": "102", "albumId": 556, "timestamp": "2026-01-02T00:00:00"}
                                ]
                            }
                        }
                    }),
                )
            }
            _ => MockResponse::json(404, json!({"error": "not found"})),
        }
    });

    let client = client_for(&server, Some("test-token"));
    let status = client.init().unwrap();
    let account = status.account.as_ref().unwrap();
    assert_eq!(account.uid, Some(42));
    assert_eq!(account.best_name(), "Кот Слушатель");
    assert!(status.has_plus());
    assert_eq!(client.cached_uid(), Some(42));

    let liked = client.liked_tracks().unwrap();
    assert_eq!(liked.uid, Some(42));
    assert_eq!(liked.revision, Some(17));
    assert_eq!(liked.tracks.len(), 2);
    assert_eq!(liked.tracks[0].track_id(), "101:555");
    assert_eq!(liked.tracks[1].track_id(), "102:556");
}

#[test]
fn tracks_endpoint_sends_repeated_track_ids() {
    let server = TestServer::start(|request| {
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/tracks");
        assert_eq!(request.header("Authorization"), Some("OAuth test-token"));
        assert!(!request.form_values("track-ids").is_empty());
        assert_eq!(
            request.form_value("with-positions").as_deref(),
            Some("true")
        );

        MockResponse::json(
            200,
            json!({
                "result": [
                    {
                        "id": "101",
                        "title": "First",
                        "artists": [{"id": 1, "name": "Artist A"}],
                        "albums": [{"id": 555, "title": "Album", "coverUri": "avatars.yandex.net/a/%%"}],
                        "durationMs": 61000,
                        "coverUri": "avatars.yandex.net/a/%%",
                        "available": true
                    },
                    {
                        "id": 102,
                        "title": "Second",
                        "artists": [{"id": 2, "name": "Artist B"}],
                        "albums": [{"id": 556, "title": "Album 2"}],
                        "durationMs": 123000
                    }
                ]
            }),
        )
    });

    let client = client_for(&server, Some("test-token"));
    let tracks = client.tracks(["101:555", "102:556"]).unwrap();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].id, "101");
    assert_eq!(tracks[0].artist_names(), "Artist A");
    assert_eq!(tracks[0].full_id(), "101:555");
    assert_eq!(
        tracks[0].cover_url(600).as_deref(),
        Some("https://avatars.yandex.net/a/600x600")
    );
    assert_eq!(tracks[1].duration(), Duration::from_secs(123));

    let requests = server.requests();
    assert_eq!(
        requests[0].form_values("track-ids"),
        vec!["101:555".to_string(), "102:556".to_string()]
    );

    let track = client.track("101:555").unwrap();
    assert_eq!(track.title.as_deref(), Some("First"));
}

#[test]
fn download_info_and_cover_download_work() {
    let server = TestServer::start(|request| {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/tracks/101:555/download-info") => {
                let host = request.header("host").unwrap();
                MockResponse::json(
                    200,
                    json!({
                        "result": [
                            {"codec": "mp3", "bitrateInKbps": 128, "gain": false, "preview": false,
                             "downloadInfoUrl": format!("http://{host}/download-info/101"), "direct": false},
                            {"codec": "mp3", "bitrateInKbps": 320, "gain": false, "preview": false,
                             "downloadInfoUrl": format!("http://{host}/download-info/101"), "direct": false}
                        ]
                    }),
                )
            }
            ("GET", "/download-info/101") => MockResponse::xml(
                "<?xml version=\"1.0\"?><download-info><host>storage.example</host><path>/get-mp3/aa/bb</path><ts>1700000000</ts><s>cafe</s></download-info>",
            ),
            ("GET", "/cover.jpg") => MockResponse::bytes(vec![0xff, 0xd8, 0xff, 0xe0]),
            _ => MockResponse::json(404, json!({"error": "not found"})),
        }
    });

    let client = client_for(&server, Some("test-token"));
    let infos = client.download_info("101:555").unwrap();
    assert_eq!(infos.len(), 2);
    assert_eq!(infos[1].bitrate_in_kbps, 320);

    let direct = client.direct_link("101:555").unwrap();
    assert!(direct.starts_with("https://storage.example/get-mp3/"));
    assert!(direct.ends_with("/1700000000/get-mp3/aa/bb"));

    let image = client
        .download_image(&format!("{}/cover.jpg", server.base_url()))
        .unwrap();
    assert_eq!(image, vec![0xff, 0xd8, 0xff, 0xe0]);
}

#[test]
fn unauthorized_and_not_found_errors_are_mapped() {
    let server = TestServer::start(|request| match request.path.as_str() {
        "/account/status" => MockResponse::json(
            401,
            json!({"error": {"name": "unauthorized", "message": "Invalid token"}}),
        ),
        "/tracks/1/download-info" => MockResponse::json(
            404,
            json!({"error": {"name": "not-found", "message": "Track not found"}}),
        ),
        _ => MockResponse::json(500, json!({"error": "unexpected"})),
    });

    let client = client_for(&server, Some("bad-token"));
    match client.account_status() {
        Err(Error::Unauthorized(message)) => {
            assert!(message.contains("Invalid token"), "message: {message}");
        }
        other => panic!("expected Unauthorized, got {other:?}"),
    }

    match client.download_info("1") {
        Err(Error::NotFound(message)) => {
            assert!(message.contains("Track not found"), "message: {message}");
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn missing_token_fails_without_network_request() {
    let client = Client::unauthenticated();
    match client.account_status() {
        Err(Error::NotAuthenticated) => {}
        other => panic!("expected NotAuthenticated, got {other:?}"),
    }
}

#[test]
fn connection_refused_is_a_network_error() {
    let client = ClientBuilder::new()
        .base_url("http://127.0.0.1:1")
        .timeout(Duration::from_secs(2))
        .token("token")
        .build();
    match client.account_status() {
        Err(Error::Network(_)) | Err(Error::Timeout) => {}
        other => panic!("expected a network error, got {other:?}"),
    }
}

#[test]
fn search_tracks_parses_paginated_results() {
    let server = TestServer::start(|request| {
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/search");
        assert_eq!(request.query_value("text"), Some("careless whisper"));
        assert_eq!(request.query_value("type"), Some("track"));
        assert_eq!(request.query_value("page"), Some("0"));
        assert_eq!(request.query_value("nocorrect"), Some("false"));
        assert_eq!(request.header("Authorization"), Some("OAuth test-token"));

        MockResponse::json(
            200,
            json!({
                "result": {
                    "text": "careless whisper",
                    "searchRequestId": "request-1",
                    "tracks": {
                        "type": "track",
                        "total": 2,
                        "perPage": 20,
                        "order": 0,
                        "results": [
                            {
                                "id": "201",
                                "title": "Careless Whisper",
                                "artists": [{"id": 1, "name": "George Michael"}],
                                "albums": [{"id": 777, "title": "Make It Big"}],
                                "durationMs": 350000,
                                "available": true
                            },
                            {
                                "id": "202",
                                "title": "Careless Whisper (Live)",
                                "artists": [{"id": 1, "name": "George Michael"}],
                                "durationMs": 370000
                            }
                        ]
                    }
                }
            }),
        )
    });

    let client = client_for(&server, Some("test-token"));
    let found = client.search_tracks("  careless whisper  ", 0).unwrap();
    assert_eq!(found.total, 2);
    assert_eq!(found.per_page, 20);
    assert_eq!(found.results.len(), 2);
    assert_eq!(found.results[0].title.as_deref(), Some("Careless Whisper"));
    assert_eq!(found.results[0].artist_names(), "George Michael");
}

#[test]
fn search_rejects_empty_queries_without_network_request() {
    let client = Client::with_token("test-token");
    match client.search_tracks("   ", 0) {
        Err(Error::BadRequest(message)) => assert!(message.contains("empty")),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn like_and_unlike_tracks_use_expected_endpoints() {
    let server = TestServer::start(|request| {
        assert_eq!(request.header("Authorization"), Some("OAuth test-token"));
        match request.path.as_str() {
            "/account/status" => {
                assert_eq!(request.method, "GET");
                MockResponse::json(200, json!({"result": {"account": {"uid": 42}}}))
            }
            "/users/42/likes/tracks/add-multiple" => {
                assert_eq!(request.method, "POST");
                assert_eq!(
                    request.form_values("track-ids"),
                    vec!["101".to_string(), "102".to_string()]
                );
                MockResponse::json(200, json!({"result": {"revision": 18}}))
            }
            "/users/42/likes/tracks/remove" => {
                assert_eq!(request.method, "POST");
                assert_eq!(
                    request.form_values("track-ids"),
                    vec!["101".to_string(), "102".to_string()]
                );
                MockResponse::json(200, json!({"result": "ok"}))
            }
            other => panic!("unexpected path: {other}"),
        }
    });

    let client = client_for(&server, Some("test-token"));
    client
        .like_tracks(&["101".to_string(), "102".to_string()])
        .unwrap();
    client
        .unlike_tracks(&["101".to_string(), "102".to_string()])
        .unwrap();

    let requests = server.requests();
    let paths: Vec<&str> = requests
        .iter()
        .map(|request| request.path.as_str())
        .collect();
    assert_eq!(
        paths,
        vec![
            "/account/status",
            "/users/42/likes/tracks/add-multiple",
            "/users/42/likes/tracks/remove"
        ]
    );
}

#[test]
fn playlists_list_and_detail_are_parsed() {
    let server = TestServer::start(|request| {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/users/42/playlists/list") => {
                assert_eq!(request.header("Authorization"), Some("OAuth test-token"));
                MockResponse::json(
                    200,
                    json!({
                        "result": [
                            {
                                "uid": 42,
                                "kind": 7,
                                "title": "Дорога домой",
                                "trackCount": 2,
                                "visibility": "public",
                                "durationMs": 441000,
                                "cover": {
                                    "type": "cover",
                                    "itemsUri": ["avatars.yandex.net/get-music-user-playlist/1/%%"]
                                },
                                "owner": {"uid": 42, "login": "listener"}
                            },
                            {
                                "uid": 42,
                                "kind": 8,
                                "title": "Приватный",
                                "trackCount": 0,
                                "visibility": "private"
                            }
                        ]
                    }),
                )
            }
            ("GET", "/users/42/playlists/7") => MockResponse::json(
                200,
                json!({
                    "result": {
                        "uid": 42,
                        "kind": 7,
                        "title": "Дорога домой",
                        "trackCount": 2,
                        "durationMs": 441000,
                        "tracks": [
                            {"id": 101, "albumId": "555", "timestamp": "2026-01-01T00:00:00"},
                            {"id": "102", "timestamp": "2026-01-02T00:00:00"}
                        ]
                    }
                }),
            ),
            other => panic!("unexpected request: {other:?}"),
        }
    });

    let client = client_for(&server, Some("test-token"));
    let playlists = client.user_playlists(42).unwrap();
    assert_eq!(playlists.len(), 2);
    assert_eq!(playlists[0].playlist_id(), "42:7");
    assert_eq!(playlists[0].display_title(), "Дорога домой");
    assert_eq!(playlists[0].track_count, Some(2));
    assert!(playlists[0].is_public());
    assert_eq!(
        playlists[0].cover_url(200).as_deref(),
        Some("https://avatars.yandex.net/get-music-user-playlist/1/200x200")
    );
    assert_eq!(
        playlists[0]
            .owner
            .as_ref()
            .and_then(|owner| owner.login.as_deref()),
        Some("listener")
    );
    assert_eq!(playlists[0].duration(), Duration::from_secs(441));

    let full = client.user_playlist(42, 7).unwrap();
    assert_eq!(full.tracks.len(), 2);
    assert_eq!(full.tracks[0].track_id(), "101:555");
    assert_eq!(full.tracks[1].track_id(), "102");
}

#[test]
fn search_artists_and_albums_are_parsed() {
    let server = TestServer::start(|request| {
        assert_eq!(request.path, "/search");
        assert_eq!(request.header("Authorization"), Some("OAuth test-token"));
        match request.query_value("type") {
            Some("artist") => MockResponse::json(
                200,
                json!({
                    "result": {
                        "artists": {
                            "type": "artist",
                            "total": 1,
                            "perPage": 10,
                            "order": 0,
                            "results": [{
                                "id": 9262,
                                "name": "Nirvana",
                                "genres": ["rock", "alternative"],
                                "cover": {
                                    "type": "pic",
                                    "uri": "avatars.yandex.net/get-music-content/1/%%"
                                }
                            }]
                        }
                    }
                }),
            ),
            Some("album") => MockResponse::json(
                200,
                json!({
                    "result": {
                        "albums": {
                            "type": "album",
                            "total": 1,
                            "perPage": 10,
                            "order": 1,
                            "results": [{
                                "id": 5576245,
                                "title": "Nevermind",
                                "year": 1991,
                                "trackCount": 12,
                                "coverUri": "avatars.yandex.net/get-music-content/2/%%",
                                "artists": [{"id": 9262, "name": "Nirvana"}]
                            }]
                        }
                    }
                }),
            ),
            other => panic!("unexpected search type: {other:?}"),
        }
    });

    let client = client_for(&server, Some("test-token"));

    let artists = client.search_artists("nirvana", 0).unwrap();
    assert_eq!(artists.total, 1);
    assert_eq!(artists.results[0].display_name(), "Nirvana");
    assert_eq!(artists.results[0].genres, vec!["rock", "alternative"]);
    assert_eq!(
        artists.results[0].cover_url(300).as_deref(),
        Some("https://avatars.yandex.net/get-music-content/1/300x300")
    );

    let albums = client.search_albums("nirvana", 0).unwrap();
    assert_eq!(albums.results[0].display_title(), "Nevermind");
    assert_eq!(albums.results[0].artist_names(), "Nirvana");
    assert_eq!(albums.results[0].year, Some(1991));
    assert_eq!(
        albums.results[0].cover_url(400).as_deref(),
        Some("https://avatars.yandex.net/get-music-content/2/400x400")
    );
}

#[test]
fn file_info_is_signed_and_lossless_download_is_preferred() {
    let server =
        TestServer::start(
            |request| match (request.method.as_str(), request.path.as_str()) {
                ("GET", "/get-file-info") => {
                    assert_eq!(request.header("Authorization"), Some("OAuth test-token"));
                    assert_eq!(
                        request.header("X-Yandex-Music-Client"),
                        Some("YandexMusicDesktopAppWindows/5.95.0")
                    );
                    assert_eq!(request.query_value("trackId"), Some("12345"));
                    assert_eq!(request.query_value("quality"), Some("lossless"));
                    assert_eq!(request.query_value("codecs"), Some("flac,flac-mp4,mp3"));
                    assert_eq!(request.query_value("transports"), Some("raw"));
                    assert!(request.query_value("ts").unwrap().parse::<u64>().is_ok());
                    assert_eq!(request.query_value("sign").unwrap().len(), 43);

                    let host = request.header("Host").expect("Host header");
                    MockResponse::json(
                        200,
                        json!({
                            "result": {
                                "downloadInfo": {
                                    "quality": "lossless",
                                    "codec": "flac-mp4",
                                    "bitrate": 0,
                                    "transport": "raw",
                                    "url": format!("http://{host}/audio/song.m4a"),
                                    "urls": [],
                                    "gain": false
                                }
                            }
                        }),
                    )
                }
                ("GET", "/audio/song.m4a") => {
                    MockResponse::bytes(b"\x00\x00\x00\x1cftypfake-mp4".to_vec())
                }
                other => panic!("unexpected request: {other:?}"),
            },
        );

    let client = client_for(&server, Some("test-token"));

    let info = client.file_info("12345").unwrap();
    let file = info.playable().expect("lossless file");
    assert_eq!(file.codec, "flac-mp4");
    assert!(file.is_lossless());

    let downloaded = client.download_track("12345").unwrap();
    assert_eq!(downloaded.codec, "flac-mp4");
    assert_eq!(downloaded.bitrate_kbps, 0);
    assert!(downloaded.bytes.starts_with(b"\x00\x00\x00\x1cftyp"));
    assert!(downloaded.direct_url.ends_with("/audio/song.m4a"));
}

#[test]
fn artist_and_album_pages_are_parsed() {
    let server = TestServer::start(|request| {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/artists/9262/brief-info") => MockResponse::json(
                200,
                json!({
                    "result": {
                        "artist": {
                            "id": 9262,
                            "name": "Nirvana",
                            "genres": ["rock", "alternative"],
                            "likesCount": 12345,
                            "counts": {"tracks": 112, "directAlbums": 33},
                            "cover": {"uri": "avatars.yandex.net/get-music-content/1/%%"}
                        }
                    }
                }),
            ),
            ("GET", "/artists/9262/tracks") => {
                assert_eq!(request.query_value("page"), Some("0"));
                assert_eq!(request.query_value("page-size"), Some("50"));
                MockResponse::json(
                    200,
                    json!({
                        "result": {
                            "tracks": [
                                {"id": 101, "title": "Come As You Are", "durationMs": 219000},
                                {"id": 102, "title": "Lithium", "durationMs": 257000}
                            ],
                            "pager": {"page": 0, "perPage": 50, "total": 112}
                        }
                    }),
                )
            }
            ("GET", "/artists/9262/direct-albums") => {
                assert_eq!(request.query_value("sort-by"), Some("year"));
                MockResponse::json(
                    200,
                    json!({
                        "result": {
                            "albums": [
                                {"id": 5576245, "title": "Nevermind", "year": 1991, "trackCount": 12}
                            ],
                            "pager": {"page": 0, "perPage": 50, "total": 33}
                        }
                    }),
                )
            }
            ("GET", "/albums/5576245/with-tracks") => MockResponse::json(
                200,
                json!({
                    "result": {
                        "id": 5576245,
                        "title": "Nevermind",
                        "year": 1991,
                        "trackCount": 2,
                        "artists": [{"id": 9262, "name": "Nirvana"}],
                        "volumes": [[
                            {"id": 201, "title": "Smells Like Teen Spirit", "durationMs": 301000},
                            {"id": 202, "title": "In Bloom", "durationMs": 254000}
                        ]]
                    }
                }),
            ),
            other => panic!("unexpected request: {other:?}"),
        }
    });

    let client = client_for(&server, Some("test-token"));

    let artist = client.artist(9262).unwrap();
    assert_eq!(artist.display_name(), "Nirvana");
    assert_eq!(artist.likes_count, Some(12345));
    assert_eq!(
        artist.counts.as_ref().map(|counts| counts.tracks),
        Some(112)
    );
    assert_eq!(
        artist.cover_url(300).as_deref(),
        Some("https://avatars.yandex.net/get-music-content/1/300x300")
    );

    let tracks = client.artist_tracks(9262, 0, 50).unwrap();
    assert_eq!(tracks.tracks.len(), 2);
    assert_eq!(tracks.pager.as_ref().map(|pager| pager.total), Some(112));

    let albums = client.artist_albums(9262, 0, 50).unwrap();
    assert_eq!(albums.albums[0].display_title(), "Nevermind");
    assert_eq!(albums.pager.as_ref().map(|pager| pager.total), Some(33));

    let album = client.album_with_tracks(5576245).unwrap();
    assert_eq!(album.display_title(), "Nevermind");
    assert_eq!(album.artist_names(), "Nirvana");
    let album_tracks = album.all_tracks();
    assert_eq!(album_tracks.len(), 2);
    assert_eq!(album_tracks[1].display_title(), "In Bloom");
    assert_eq!(album.duration(), Duration::from_secs(555));
}

#[test]
fn wave_station_returns_tracks_and_continues_from_queue() {
    let server = TestServer::start(|request| {
        assert_eq!(request.method, "GET", "wave endpoint must use GET");
        assert_eq!(request.path, "/rotor/station/user:onyourwave/tracks");
        assert_eq!(request.query_value("settings2"), Some("true"));
        match request.query_value("queue") {
            None => MockResponse::json(
                200,
                json!({
                    "result": {
                        "batchId": "batch-1",
                        "radioSessionId": "session-1",
                        "sequence": [
                            {"type": "track", "liked": false, "track": {"id": 1, "title": "One"}},
                            {"type": "track", "liked": true, "track": {"id": 2, "title": "Two"}}
                        ]
                    }
                }),
            ),
            Some("2") => MockResponse::json(
                200,
                json!({
                    "result": {
                        "batchId": "batch-2",
                        "sequence": [
                            {"type": "track", "liked": false, "track": {"id": 3, "title": "Three"}}
                        ]
                    }
                }),
            ),
            other => panic!("unexpected queue: {other:?}"),
        }
    });

    let client = client_for(&server, Some("test-token"));
    let first = client.wave_tracks(None).unwrap();
    assert_eq!(first.batch_id.as_deref(), Some("batch-1"));
    assert_eq!(first.tracks().len(), 2);
    assert_eq!(first.tracks()[1].display_title(), "Two");

    let next = client.wave_tracks(Some("2")).unwrap();
    assert_eq!(next.tracks()[0].display_title(), "Three");
}
