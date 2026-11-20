//! Window shell: title bar, sidebar and player bar.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn app_shell(
        &mut self,
        theme: &Theme,
        cx: &mut Context<Self>,
        main: AnyElement,
    ) -> AnyElement {
        let drawer = if self.queue_open {
            Some(self.queue_drawer(theme, cx))
        } else {
            None
        };

        let mut content_row = div()
            .relative()
            .flex()
            .flex_1()
            .min_h(px(0.0))
            .overflow_hidden()
            .child(self.sidebar(theme, cx))
            .child(main);

        if let Some(drawer) = drawer {
            content_row = content_row.child(drawer);
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .child(content_row)
            .child(self.player_bar(theme, cx))
            .into_any_element()
    }

    pub(crate) fn sidebar(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let name = self
            .account()
            .map(|account| account.best_name().to_string())
            .unwrap_or_else(|| "Слушатель".into());
        let login = self
            .account()
            .and_then(|account| account.login.clone())
            .unwrap_or_default();

        let profile_active = matches!(self.screen, Screen::Profile);
        let liked_active = matches!(self.screen, Screen::Liked);
        let wave_active = matches!(self.screen, Screen::Wave);
        let now_active = matches!(self.screen, Screen::NowPlaying);
        let search_active = matches!(self.screen, Screen::Search);
        let playlists_active = matches!(self.screen, Screen::Playlists | Screen::Playlist);
        let accent = theme.colors.accent;
        let secondary = theme.colors.text_secondary;
        let hover_bg = theme.colors.surface_hover;

        let nav_liked = div()
            .id("nav-liked")
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(42.0))
            .px(px(space::MD))
            .rounded(px(10.0))
            .cursor_pointer()
            .text_size(px(text::BASE))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .when(liked_active, |element| {
                element.bg(theme.colors.accent_soft).text_color(accent)
            })
            .when(!liked_active, |element| {
                element
                    .text_color(secondary)
                    .hover(move |style| style.bg(hover_bg))
            })
            .child(Icon::new(icons::HEART).size(17.0).color(if liked_active {
                accent
            } else {
                secondary
            }))
            .child("Любимые")
            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.open_liked(cx)));

        let nav_now = div()
            .id("nav-now-playing")
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(42.0))
            .px(px(space::MD))
            .rounded(px(10.0))
            .cursor_pointer()
            .text_size(px(text::BASE))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .when(now_active, |element| {
                element.bg(theme.colors.accent_soft).text_color(accent)
            })
            .when(!now_active, |element| {
                element
                    .text_color(secondary)
                    .hover(move |style| style.bg(hover_bg))
            })
            .child(Icon::new(icons::MUSIC).size(17.0).color(if now_active {
                accent
            } else {
                secondary
            }))
            .child("Сейчас играет")
            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.open_now_playing(cx);
            }));

        let nav_wave = div()
            .id("nav-wave")
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(42.0))
            .px(px(space::MD))
            .rounded(px(10.0))
            .cursor_pointer()
            .text_size(px(text::BASE))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .when(wave_active, |element| {
                element.bg(theme.colors.accent_soft).text_color(accent)
            })
            .when(!wave_active, |element| {
                element
                    .text_color(secondary)
                    .hover(move |style| style.bg(hover_bg))
            })
            .child(Icon::new(icons::SPARKLES).size(17.0).color(if wave_active {
                accent
            } else {
                secondary
            }))
            .child("Моя волна")
            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.open_wave(cx);
            }));

        let nav_search = div()
            .id("nav-search")
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(42.0))
            .px(px(space::MD))
            .rounded(px(10.0))
            .cursor_pointer()
            .text_size(px(text::BASE))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .when(search_active, |element| {
                element.bg(theme.colors.accent_soft).text_color(accent)
            })
            .when(!search_active, |element| {
                element
                    .text_color(secondary)
                    .hover(move |style| style.bg(hover_bg))
            })
            .child(Icon::new(icons::SEARCH).size(17.0).color(if search_active {
                accent
            } else {
                secondary
            }))
            .child("Поиск")
            .on_click(
                cx.listener(|this, _: &gpui::ClickEvent, window, cx| this.open_search(window, cx)),
            );

        let nav_playlists = div()
            .id("nav-playlists")
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(42.0))
            .px(px(space::MD))
            .rounded(px(10.0))
            .cursor_pointer()
            .text_size(px(text::BASE))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .when(playlists_active, |element| {
                element.bg(theme.colors.accent_soft).text_color(accent)
            })
            .when(!playlists_active, |element| {
                element
                    .text_color(secondary)
                    .hover(move |style| style.bg(hover_bg))
            })
            .child(
                Icon::new(icons::PLAYLIST)
                    .size(17.0)
                    .color(if playlists_active { accent } else { secondary }),
            )
            .child("Плейлисты")
            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.open_playlists(cx);
            }));

        let profile_zone = div()
            .id("account-profile")
            .flex()
            .items_center()
            .gap(px(space::SM))
            .flex_1()
            .min_w(px(0.0))
            .rounded(px(8.0))
            .cursor_pointer()
            .hover(|style| style.bg(theme.colors.surface_hover))
            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.open_profile(cx);
            }))
            .child(Avatar::new(name.clone()).size(34.0).ring(true))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(1.0))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_muted)
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(if login.is_empty() {
                                "демо-режим".to_string()
                            } else {
                                login.clone()
                            }),
                    ),
            );

        let account_card = div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .p(px(space::SM))
            .rounded(px(12.0))
            .bg(theme.colors.surface)
            .border_1()
            .border_color(if profile_active {
                theme.colors.accent
            } else {
                theme.colors.border
            })
            .child(profile_zone)
            .child(
                IconButton::new("sidebar-logout", icons::LOGOUT)
                    .size(30.0)
                    .icon_size(14.0)
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                        this.logout(cx);
                    })),
            );

        let mut sidebar = div()
            .w(px(244.0))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(space::LG))
            .p(px(space::LG))
            .bg(theme.colors.sidebar)
            .border_r_1()
            .border_color(theme.colors.border)
            .child(brand(theme, false));

        if self.demo {
            sidebar = sidebar.child(
                div()
                    .flex()
                    .child(Badge::new("ДЕМО-РЕЖИМ").tone(BadgeTone::Accent))
                    .child(div().flex_1()),
            );
        }

        sidebar = sidebar
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::XS))
                    .child(section_label(theme, "МЕНЮ"))
                    .child(nav_liked)
                    .child(nav_wave)
                    .child(nav_now)
                    .child(nav_search)
                    .child(nav_playlists),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .child(account_card)
                    .child(
                        div()
                            .px(px(space::SM))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("rsyameow 0.1.0")
                            .child("gpui"),
                    ),
            );

        sidebar.into_any_element()
    }

    pub(crate) fn player_bar(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let track = self.player.track.clone();
        let title = track
            .as_ref()
            .map(|track| track.display_title().to_string())
            .unwrap_or_default();
        let artist = track
            .as_ref()
            .map(|track| track.artist_names())
            .unwrap_or_default();
        let duration = track
            .as_ref()
            .map(|track| format::duration_or_dash(track.duration()))
            .unwrap_or_else(|| "0:00".into());
        let seek_preview = self.seek_preview;
        let progress = seek_preview.unwrap_or_else(|| self.player.progress());
        let elapsed = if let Some(fraction) = seek_preview {
            track
                .as_ref()
                .map(|track| format::duration(track.duration().mul_f32(fraction)))
                .unwrap_or_else(|| "0:00".into())
        } else {
            format::duration(self.player.position)
        };
        let artist_link = track
            .as_ref()
            .and_then(|track| track.artists.first().cloned());

        let view = cx.entity();
        let preview_view = cx.entity();
        let volume_view = cx.entity();
        let bar = PlayerBar::new()
            .track(title, artist)
            .explicit(
                self.player
                    .track
                    .as_ref()
                    .is_some_and(|track| track.explicit.unwrap_or(false)),
            )
            .cover(self.player.cover.clone())
            .progress(progress, elapsed, duration)
            .playing(self.player.playing)
            .loading(self.player.loading)
            .quality(self.player.quality.clone())
            .error(self.player.error.clone())
            .volume(self.volume)
            .compact(self.viewport_width < 1100.0)
            .liked(self.is_current_liked())
            .queue_open(self.queue_open)
            .on_like(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.toggle_like(cx);
            }))
            .on_queue(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.toggle_queue(cx);
            }))
            .on_track(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.open_now_playing(cx);
            }))
            .on_artist(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                if let Some(artist) = artist_link.clone()
                    && let Some(id) = artist.id
                {
                    this.open_artist(id, Some(artist), cx);
                }
            }))
            .on_mute(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.toggle_mute(cx);
            }))
            .on_volume(move |volume, _, cx| {
                volume_view.update(cx, |this, cx| this.set_volume(volume, cx));
            })
            .on_toggle(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.toggle_playback(cx);
            }))
            .on_previous(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.play_previous(cx);
            }))
            .on_next(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.play_next(cx);
            }))
            .on_seek_preview(move |fraction, _, cx| {
                preview_view.update(cx, |this, cx| {
                    this.seek_preview = Some(fraction);
                    cx.notify();
                });
            })
            .on_seek(move |fraction, _, cx| {
                view.update(cx, |this, cx| {
                    if this.seek_preview.is_some() {
                        this.seek_to(fraction, cx);
                    }
                });
            });

        let _ = theme;
        bar.into_any_element()
    }

    pub(crate) fn account(&self) -> Option<&yandex_music::Account> {
        self.profile
            .as_ref()
            .and_then(|profile| profile.status.account.as_ref())
    }
}

pub(crate) fn screen_title(screen: &Screen) -> &'static str {
    match screen {
        Screen::Login => "Вход",
        Screen::Loading => "Загрузка",
        Screen::Profile => "Профиль",
        Screen::Liked => "Любимые",
        Screen::Search => "Поиск",
        Screen::Playlists | Screen::Playlist => "Плейлисты",
        Screen::Artist => "Артист",
        Screen::ArtistTracks => "Треки артиста",
        Screen::ArtistAlbums => "Альбомы артиста",
        Screen::Album => "Альбом",
        Screen::Track => "Трек",
        Screen::NowPlaying => "Сейчас играет",
        Screen::Wave => "Моя волна",
        Screen::LoadError(_) => "Ошибка",
    }
}

/// A slim client-side titlebar with a draggable area and window controls.
pub(crate) fn title_bar(screen: &Screen, window: &mut Window, theme: &Theme) -> AnyElement {
    let maximized = window.is_maximized();

    let controls = div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .child(window_control(
            "window-minimize",
            icons::WINDOW_MINIMIZE,
            theme.colors.text_secondary,
            theme.colors.surface_hover,
            |_, window, _| window.minimize_window(),
        ))
        .child(window_control(
            "window-maximize",
            if maximized {
                icons::WINDOW_RESTORE
            } else {
                icons::WINDOW_MAXIMIZE
            },
            theme.colors.text_secondary,
            theme.colors.surface_hover,
            |_, window, _| window.zoom_window(),
        ))
        .child(window_control(
            "window-close",
            icons::CLOSE,
            theme.colors.white,
            theme.colors.danger,
            |_, window, _| window.remove_window(),
        ));

    div()
        .flex()
        .items_center()
        .h(px(38.0))
        .w_full()
        .flex_shrink_0()
        .pl(px(10.0))
        .pr(px(8.0))
        .bg(theme.colors.background)
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .size(px(18.0))
                .rounded(px(6.0))
                .bg(theme.colors.accent)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(icons::MUSIC)
                        .size(9.0)
                        .color(theme.colors.accent_text),
                ),
        )
        .child(
            div()
                .pl(px(space::SM))
                .pr(px(space::MD))
                .flex()
                .items_center()
                .gap(px(space::SM))
                .child(
                    div()
                        .text_size(px(text::SM))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text)
                        .child("rsyameow"),
                )
                .child(
                    div()
                        .size(px(3.0))
                        .rounded_full()
                        .bg(theme.colors.text_muted),
                )
                .child(
                    div()
                        .text_size(px(text::SM))
                        .text_color(theme.colors.text_muted)
                        .child(screen_title(screen)),
                ),
        )
        .child(
            div()
                .id("window-drag-region")
                .flex_1()
                .h_full()
                .cursor(CursorStyle::Arrow)
                .on_mouse_down(MouseButton::Left, |event, window, _| {
                    if event.click_count == 1 {
                        window.start_window_move();
                    }
                })
                .on_mouse_down(MouseButton::Right, |event, window, _| {
                    window.show_window_menu(event.position);
                })
                .on_click(|event, window, _| {
                    if event.click_count() == 2 {
                        window.zoom_window();
                    }
                }),
        )
        .child(controls)
        .into_any_element()
}

pub(crate) fn window_control(
    id: &'static str,
    icon: &'static str,
    foreground: Rgba,
    hover_background: Rgba,
    handler: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(36.0))
        .h(px(28.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(move |style| style.bg(hover_background))
        .on_click(handler)
        .child(Icon::new(icon).size(13.0).color(foreground))
}

pub(crate) fn resize_handle(
    id: &'static str,
    edge: ResizeEdge,
    cursor: CursorStyle,
) -> Stateful<Div> {
    div()
        .id(id)
        .cursor(cursor)
        .on_mouse_down(MouseButton::Left, move |_, window, _| {
            window.start_window_resize(edge);
        })
}
