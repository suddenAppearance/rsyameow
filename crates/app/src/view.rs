//! Rendering for [`RootView`].

use crate::app::{
    CloseQueue, CloseWindow, FocusSearch, NextTrack, PreviousTrack, RootView, Screen, SearchKind,
    ToggleMute, TogglePlayback, ToggleQueue, plan_name, plus_until, registered_year,
};
use crate::format;
use gpui::{
    AnyElement, Context, CursorStyle, Div, ElementId, Focusable as _, Image, ImageSource,
    IntoElement, MouseButton, ObjectFit, Render, ResizeEdge, Rgba, SharedString, Stateful, Window,
    div, img, prelude::*, px, relative, rgba,
};
use rsyameow_ui::{
    Avatar, Badge, BadgeTone, Button, ButtonSize, ButtonVariant, Icon, IconButton, PlayerBar,
    Spinner, Stat, Theme, TrackRow, icons, space, text,
};
use std::sync::Arc;
use yandex_music::{Playlist, Status};

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width);
        let height = f32::from(viewport.height);
        if (width - self.viewport_width).abs() > 1.0 || (height - self.viewport_height).abs() > 1.0
        {
            self.viewport_width = width;
            self.viewport_height = height;
            self.schedule_window_size_save(cx);
        }

        let theme = Theme::get(cx).clone();
        let screen = self.screen.clone();

        let content: AnyElement = match &screen {
            Screen::Login => self.login_view(&theme, cx),
            Screen::Loading => self.loading_view(&theme),
            Screen::LoadError(message) => self.load_error_view(message, &theme, cx),
            Screen::Profile => self.profile_view(&theme, cx),
            Screen::Search => self.search_view(&theme, cx),
            Screen::Playlists => self.playlists_view(&theme, cx),
            Screen::Playlist => self.playlist_detail_view(&theme, cx),
        };

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .key_context("rsyameow")
            .on_action(cx.listener(|this, _: &TogglePlayback, _, cx| this.toggle_playback(cx)))
            .on_action(cx.listener(|this, _: &NextTrack, _, cx| this.play_next(cx)))
            .on_action(cx.listener(|this, _: &PreviousTrack, _, cx| this.play_previous(cx)))
            .on_action(cx.listener(|this, _: &ToggleMute, _, cx| this.toggle_mute(cx)))
            .on_action(cx.listener(|this, _: &ToggleQueue, _, cx| this.toggle_queue(cx)))
            .on_action(cx.listener(|this, _: &CloseQueue, _, cx| this.close_queue(cx)))
            .on_action(
                cx.listener(|this, _: &FocusSearch, window, cx| this.open_search(window, cx)),
            )
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .bg(theme.colors.background)
            .text_color(theme.colors.text)
            .font_family(theme.fonts.ui.clone())
            .child(title_bar(&screen, window, &theme))
            .child(div().flex_1().min_h(px(0.0)).child(content))
            .child(
                resize_handle("resize-top", ResizeEdge::Top, CursorStyle::ResizeUpDown)
                    .absolute()
                    .top(px(0.0))
                    .left(px(8.0))
                    .right(px(8.0))
                    .h(px(5.0)),
            )
            .child(
                resize_handle(
                    "resize-bottom",
                    ResizeEdge::Bottom,
                    CursorStyle::ResizeUpDown,
                )
                .absolute()
                .bottom(px(0.0))
                .left(px(8.0))
                .right(px(8.0))
                .h(px(5.0)),
            )
            .child(
                resize_handle(
                    "resize-left",
                    ResizeEdge::Left,
                    CursorStyle::ResizeLeftRight,
                )
                .absolute()
                .left(px(0.0))
                .top(px(8.0))
                .bottom(px(8.0))
                .w(px(5.0)),
            )
            .child(
                resize_handle(
                    "resize-right",
                    ResizeEdge::Right,
                    CursorStyle::ResizeLeftRight,
                )
                .absolute()
                .right(px(0.0))
                .top(px(8.0))
                .bottom(px(8.0))
                .w(px(5.0)),
            )
            .child(
                resize_handle(
                    "resize-top-left",
                    ResizeEdge::TopLeft,
                    CursorStyle::ResizeUpLeftDownRight,
                )
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .w(px(8.0))
                .h(px(8.0)),
            )
            .child(
                resize_handle(
                    "resize-top-right",
                    ResizeEdge::TopRight,
                    CursorStyle::ResizeUpRightDownLeft,
                )
                .absolute()
                .top(px(0.0))
                .right(px(0.0))
                .w(px(8.0))
                .h(px(8.0)),
            )
            .child(
                resize_handle(
                    "resize-bottom-left",
                    ResizeEdge::BottomLeft,
                    CursorStyle::ResizeUpRightDownLeft,
                )
                .absolute()
                .bottom(px(0.0))
                .left(px(0.0))
                .w(px(8.0))
                .h(px(8.0)),
            )
            .child(
                resize_handle(
                    "resize-bottom-right",
                    ResizeEdge::BottomRight,
                    CursorStyle::ResizeUpLeftDownRight,
                )
                .absolute()
                .bottom(px(0.0))
                .right(px(0.0))
                .w(px(8.0))
                .h(px(8.0)),
            )
    }
}

impl RootView {
    // ------------------------------------------------------------------ Login

    fn login_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let mut card = div()
            .relative()
            .w(px(500.0))
            .max_w(relative(0.92))
            .rounded(px(24.0))
            .bg(theme.colors.surface)
            .border_1()
            .border_color(theme.colors.border)
            .p(px(32.0))
            .flex()
            .flex_col()
            .gap(px(space::LG))
            .child(brand(theme, true));

        if let Some(error) = self.login_error.clone() {
            card = card.child(error_banner(theme, &error));
        }

        if let Some(flow) = &self.device {
            let user_code = flow.code.user_code.clone();
            let verification_url = flow.code.verification_url.clone();
            let copied = flow.copied;

            card = card
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(space::XS))
                        .child(
                            div()
                                .text_size(px(text::XL))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child("Подтвердите вход"),
                        )
                        .child(
                            div()
                                .text_size(px(text::SM))
                                .text_color(theme.colors.text_secondary)
                                .child(
                                    "Откройте страницу Яндекса и введите этот код, чтобы связать аккаунт.",
                                )
                                .child(
                                    div()
                                        .text_size(px(text::XS))
                                        .text_color(theme.colors.text_muted)
                                        .child(verification_url.clone()),
                                ),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .rounded(px(16.0))
                        .bg(theme.colors.background)
                        .border_1()
                        .border_color(theme.colors.border_strong)
                        .px(px(space::XL))
                        .py(px(space::XL))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(space::SM))
                        .child(
                            div()
                                .font_family(theme.fonts.mono.clone())
                                .text_size(px(32.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme.colors.accent)
                                .child(user_code),
                        )
                        .child(
                            div()
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child("Код действует несколько минут"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(space::SM))
                        .child(
                            Button::new("open-device-url", "Открыть страницу")
                                .variant(ButtonVariant::Primary)
                                .icon(icons::EXTERNAL)
                                .grow()
                                .on_click(move |_, _, cx| cx.open_url(&verification_url)),
                        )
                        .child(
                            Button::new(
                                "copy-device-code",
                                if copied { "Скопировано" } else { "Скопировать" },
                            )
                            .variant(ButtonVariant::Secondary)
                            .icon(if copied { icons::CHECK } else { icons::COPY })
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.copy_device_code(cx);
                            })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(space::SM))
                        .text_size(px(text::SM))
                        .text_color(theme.colors.text_secondary)
                        .child(Spinner::new().size(14.0).color(theme.colors.accent))
                        .child("Ожидаем подтверждение в Яндексе…"),
                )
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .child(
                            Button::new("cancel-device", "Отменить")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Small)
                                .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                    this.cancel_device_flow(cx);
                                })),
                        ),
                );
        } else {
            card = card
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(space::SM))
                        .child(
                            div()
                                .text_size(px(text::DISPLAY))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child("Вход в Яндекс Музыку"),
                        )
                        .child(
                            div()
                                .text_size(px(text::BASE))
                                .text_color(theme.colors.text_secondary)
                                .child(
                                    "Подключите аккаунт, чтобы слушать любимые треки в удобном клиенте.",
                                ),
                        ),
                )
                .child(
                    Button::new("start-device-auth", "Получить код")
                        .variant(ButtonVariant::Primary)
                        .size(ButtonSize::Large)
                        .icon(icons::SPARKLES)
                        .loading(self.auth_busy)
                        .full_width()
                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                            this.start_device_flow(cx);
                        })),
                )
                .child(divider(theme, "или"))
                .child(
                    Button::new(
                        "toggle-token-input",
                        if self.show_token_input {
                            "Скрыть ввод токена"
                        } else {
                            "Войти по токену"
                        },
                    )
                    .variant(ButtonVariant::Ghost)
                    .full_width()
                    .on_click(cx.listener(
                        |this, _: &gpui::ClickEvent, window, cx| {
                            this.show_token_input = !this.show_token_input;
                            if this.show_token_input {
                                let focus = this.token_input.read(cx).focus_handle(cx);
                                window.focus(&focus);
                            }
                            cx.notify();
                        },
                    )),
                );

            if self.show_token_input {
                card = card.child(self.token_input.clone()).child(
                    Button::new("login-with-token", "Войти с токеном")
                        .variant(ButtonVariant::Primary)
                        .icon(icons::USER)
                        .full_width()
                        .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                            let token = this.token_input.read(cx).text().to_string();
                            this.begin_login_with_token(token, true, cx);
                        })),
                );
            }

            card = card
                .child(
                    div().flex().gap(px(space::SM)).child(
                        Button::new("start-demo", "Посмотреть демо")
                            .variant(ButtonVariant::Soft)
                            .icon(icons::PLAY)
                            .grow()
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.start_demo(cx);
                            })),
                    ),
                )
                .child(
                    div()
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child("Демо-режим показывает интерфейс с вымышленными треками, без сети."),
                );
        }

        card = card.child(
            div()
                .pt(px(space::SM))
                .border_t_1()
                .border_color(theme.colors.border)
                .text_size(px(text::XS))
                .text_color(theme.colors.text_muted)
                .child(
                    "Неофициальный клиент. OAuth-код вводится на стороне Яндекса — пароль не покидает Яндекс.",
                ),
        );

        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.background)
            .child(blob(theme.colors.accent_soft, -160.0, -120.0, 460.0))
            .child(blob(rgba(0x6d5dfc22), 980.0, 520.0, 420.0))
            .child(card)
            .into_any_element()
    }

    // --------------------------------------------------------------- Loading

    fn loading_view(&mut self, theme: &Theme) -> AnyElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.background)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(space::LG))
                    .child(brand(theme, false))
                    .child(Spinner::new().size(26.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Загружаем профиль и любимые треки…"),
                    )
                    .child(
                        div()
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_muted)
                            .child("Это займёт пару секунд"),
                    ),
            )
            .into_any_element()
    }

    fn load_error_view(
        &mut self,
        message: &str,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.background)
            .child(
                div()
                    .w(px(470.0))
                    .rounded(px(20.0))
                    .bg(theme.colors.surface)
                    .border_1()
                    .border_color(theme.colors.border)
                    .p(px(28.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(space::LG))
                    .child(
                        div()
                            .size(px(56.0))
                            .rounded(px(18.0))
                            .bg(theme.colors.danger_soft)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(icons::WARNING)
                                    .size(26.0)
                                    .color(theme.colors.danger),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(text::XL))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Не удалось загрузить профиль"),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .text_center()
                            .child(message.to_string()),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(space::SM))
                            .w_full()
                            .child(
                                Button::new("retry-profile", "Повторить")
                                    .variant(ButtonVariant::Primary)
                                    .full_width()
                                    .icon(icons::REFRESH)
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.retry_profile(cx);
                                    })),
                            )
                            .child(
                                Button::new("logout-from-error", "Выйти")
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.logout(cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    // --------------------------------------------------------------- Profile

    fn app_shell(&mut self, theme: &Theme, cx: &mut Context<Self>, main: AnyElement) -> AnyElement {
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

    fn queue_drawer(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let (start, end) = self.queue_window();
        let total = self.queue.len();
        let current_index = self.queue_index;
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;
        let width = if self.viewport_width < 1100.0 {
            300.0
        } else {
            360.0
        };

        let mut list = div().flex().flex_col().gap(px(2.0));

        for (offset, id) in self.queue[start..end].iter().enumerate() {
            let index = start + offset;
            let track = self.queue_track_cache.get(id).cloned();
            let current = current_index == Some(index)
                || track
                    .as_ref()
                    .is_some_and(|track| current_id.as_deref() == Some(track.id.as_str()));

            let leading = if current {
                div()
                    .w(px(20.0))
                    .flex()
                    .items_end()
                    .justify_center()
                    .gap(px(2.0))
                    .child(
                        div()
                            .w(px(3.0))
                            .h(px(7.0))
                            .rounded_full()
                            .bg(theme.colors.accent),
                    )
                    .child(
                        div()
                            .w(px(3.0))
                            .h(px(13.0))
                            .rounded_full()
                            .bg(theme.colors.accent),
                    )
                    .child(
                        div()
                            .w(px(3.0))
                            .h(if playing { px(10.0) } else { px(5.0) })
                            .rounded_full()
                            .bg(theme.colors.accent),
                    )
                    .into_any_element()
            } else {
                div()
                    .w(px(20.0))
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .text_right()
                    .child(format!("{}", index + 1))
                    .into_any_element()
            };

            let title = track
                .as_ref()
                .map(|track| track.display_title().to_string())
                .unwrap_or_else(|| "Загрузка…".to_string());
            let artist = track
                .as_ref()
                .map(|track| track.artist_names())
                .unwrap_or_default();
            let duration = track
                .as_ref()
                .map(|track| format::duration_or_dash(track.duration()))
                .unwrap_or_default();

            let row = div()
                .id(ElementId::Name(SharedString::from(format!(
                    "queue-row-{index}"
                ))))
                .flex()
                .items_center()
                .gap(px(space::SM))
                .h(px(50.0))
                .px(px(space::SM))
                .rounded(px(8.0))
                .cursor_pointer()
                .when(current, |element| element.bg(theme.colors.accent_soft))
                .hover(|style| style.bg(theme.colors.surface_hover))
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.play_queue_at(index, cx);
                }))
                .child(leading)
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
                                .font_weight(if current {
                                    gpui::FontWeight::SEMIBOLD
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .text_color(if current {
                                    theme.colors.accent
                                } else {
                                    theme.colors.text
                                })
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(artist),
                        ),
                )
                .child(
                    div()
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .child(duration),
                );

            list = list.child(row);
        }

        let body = if self.queue.is_empty() {
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(space::SM))
                .p(px(space::XL))
                .child(
                    Icon::new(icons::QUEUE)
                        .size(28.0)
                        .color(theme.colors.text_muted),
                )
                .child(
                    div()
                        .text_size(px(text::SM))
                        .text_color(theme.colors.text_secondary)
                        .child("Очередь пуста"),
                )
                .child(
                    div()
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child("Включите трек, и здесь появится очередь воспроизведения."),
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(space::SM))
                .p(px(space::SM))
                .child(list)
                .when(total > end - start, |element| {
                    element.child(
                        div()
                            .pt(px(space::SM))
                            .text_size(px(text::XS))
                            .text_color(theme.colors.text_muted)
                            .text_center()
                            .child(format!("Показано {} из {total}", end - start)),
                    )
                })
                .into_any_element()
        };

        div()
            .absolute()
            .top(px(0.0))
            .right(px(0.0))
            .h_full()
            .w(px(width))
            .flex()
            .flex_col()
            .bg(theme.colors.surface)
            .border_l_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(space::LG))
                    .py(px(space::MD))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::SM))
                            .child(section_title(theme, "Очередь"))
                            .child(Badge::new(format!("{total}")).tone(BadgeTone::Neutral)),
                    )
                    .child(
                        IconButton::new("close-queue", icons::CLOSE)
                            .size(30.0)
                            .icon_size(14.0)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.close_queue(cx)
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .id("queue-scroll")
                    .overflow_y_scroll()
                    .child(body),
            )
            .into_any_element()
    }

    fn profile_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let main = self.main_content(theme, cx);
        self.app_shell(theme, cx, main)
    }

    fn search_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let main = self.search_content(theme, cx);
        self.app_shell(theme, cx, main)
    }

    fn sidebar(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let name = self
            .account()
            .map(|account| account.best_name().to_string())
            .unwrap_or_else(|| "Слушатель".into());
        let login = self
            .account()
            .and_then(|account| account.login.clone())
            .unwrap_or_default();

        let profile_active = matches!(self.screen, Screen::Profile);
        let search_active = matches!(self.screen, Screen::Search);
        let playlists_active = matches!(self.screen, Screen::Playlists | Screen::Playlist);
        let accent = theme.colors.accent;
        let secondary = theme.colors.text_secondary;
        let hover_bg = theme.colors.surface_hover;

        let nav_profile = div()
            .id("nav-profile")
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(42.0))
            .px(px(space::MD))
            .rounded(px(10.0))
            .cursor_pointer()
            .text_size(px(text::BASE))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .when(profile_active, |element| {
                element.bg(theme.colors.accent_soft).text_color(accent)
            })
            .when(!profile_active, |element| {
                element
                    .text_color(secondary)
                    .hover(move |style| style.bg(hover_bg))
            })
            .child(Icon::new(icons::USER).size(17.0).color(if profile_active {
                accent
            } else {
                secondary
            }))
            .child("Профиль")
            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| this.open_profile(cx)));

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

        let mut account_card = div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .p(px(space::SM))
            .rounded(px(12.0))
            .bg(theme.colors.surface)
            .border_1()
            .border_color(theme.colors.border)
            .cursor_pointer()
            .hover(|style| style.bg(theme.colors.surface_hover));

        account_card = account_card.child(Avatar::new(name.clone()).size(34.0).ring(true));
        account_card = account_card.child(
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

        account_card = account_card.child(
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
                    .child(nav_profile)
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

    fn main_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(profile) = self.profile.as_ref() else {
            return div().flex_1().into_any_element();
        };

        let status = &profile.status;
        let account = status.account.as_ref();
        let name = account
            .map(|account| account.best_name().to_string())
            .unwrap_or_else(|| "Слушатель".into());
        let login = account
            .and_then(|account| account.login.clone())
            .unwrap_or_default();
        let liked_total = profile.total_liked;
        let _ = profile;

        let header = div()
            .flex()
            .items_end()
            .justify_between()
            .gap(px(space::LG))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(text::DISPLAY))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Профиль"),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Аккаунт, подписка и любимая музыка в одном месте"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .child(
                        IconButton::new("refresh-profile", icons::REFRESH)
                            .size(38.0)
                            .icon_size(16.0)
                            .variant(ButtonVariant::Secondary)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                if this.demo {
                                    this.start_demo(cx);
                                } else {
                                    this.retry_profile(cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("random-track", "Случайный трек")
                            .variant(ButtonVariant::Primary)
                            .icon(icons::SHUFFLE)
                            .size(ButtonSize::Medium)
                            .loading(self.player.loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.play_random(cx);
                            })),
                    ),
            );

        let hero = self.hero_card(theme, status, &name, &login, liked_total);
        let details = self.details_row(theme, status);
        let liked = self.liked_section(theme, cx);

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .id("profile-scroll")
            .overflow_y_scroll()
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(1180.0))
                            .p(px(space::XL))
                            .flex()
                            .flex_col()
                            .gap(px(space::LG))
                            .child(header)
                            .child(hero)
                            .child(details)
                            .child(liked)
                            .child(
                                div()
                                    .pt(px(space::SM))
                                    .text_size(px(text::XS))
                                    .text_color(theme.colors.text_muted)
                                    .child(
                                        "Неофициальный клиент Яндекс Музыки. Треки загружаются напрямую из API и не сохраняются на диск.",
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn hero_card(
        &self,
        theme: &Theme,
        status: &Status,
        name: &str,
        login: &str,
        liked_total: usize,
    ) -> AnyElement {
        let has_plus = status.has_plus();
        let plus_date = plus_until(status);
        let year = registered_year(status);

        let badges = div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .when(has_plus, |element| {
                element.child(
                    Badge::new("PLUS")
                        .tone(BadgeTone::Accent)
                        .icon(icons::SPARKLES),
                )
            })
            .when(!has_plus, |element| {
                element.child(Badge::new("Без Plus").tone(BadgeTone::Neutral))
            })
            .when(self.demo, |element| {
                element.child(Badge::new("Демо").tone(BadgeTone::Outline))
            });

        div()
            .relative()
            .overflow_hidden()
            .w_full()
            .rounded(px(20.0))
            .bg(theme.colors.surface)
            .border_1()
            .border_color(theme.colors.border)
            .p(px(space::XL))
            .child(
                div()
                    .absolute()
                    .top(px(-120.0))
                    .right(px(-40.0))
                    .size(px(300.0))
                    .rounded_full()
                    .bg(theme.colors.accent_soft),
            )
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(space::XL))
                    .child(Avatar::new(name.to_string()).size(96.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(space::SM))
                            .flex_1()
                            .min_w(px(0.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(space::MD))
                                    .child(
                                        div()
                                            .text_size(px(34.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .line_height(px(38.0))
                                            .min_w(px(0.0))
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .whitespace_nowrap()
                                            .child(name.to_string()),
                                    )
                                    .child(badges),
                            )
                            .child(
                                div()
                                    .text_size(px(text::BASE))
                                    .text_color(theme.colors.text_secondary)
                                    .child(if login.is_empty() {
                                        "Яндекс Музыка".to_string()
                                    } else {
                                        format!("@{login}")
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(space::XL))
                                    .pt(px(space::SM))
                                    .child(
                                        Stat::new(
                                            format::plural(
                                                liked_total as u64,
                                                "трек",
                                                "трека",
                                                "треков",
                                            ),
                                            "в коллекции",
                                        )
                                        .accent(has_plus),
                                    )
                                    .child(divider_vertical(theme))
                                    .child(Stat::new(plus_date, "Plus активен до"))
                                    .child(divider_vertical(theme))
                                    .child(Stat::new(year, "год с нами")),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn details_row(&self, theme: &Theme, status: &Status) -> AnyElement {
        let account = status.account.as_ref();
        let has_plus = status.has_plus();
        let until = plus_until(status);
        let plan = plan_name(status);
        let region = account
            .and_then(|account| account.region)
            .map(|region| region.to_string())
            .unwrap_or_else(|| "—".into());
        let registered = account
            .and_then(|account| account.registered_at.as_deref())
            .map(format::date)
            .unwrap_or_else(|| "—".into());
        let uid = account
            .and_then(|account| account.uid)
            .map(|uid| uid.to_string())
            .unwrap_or_else(|| "—".into());
        let email = status
            .default_email
            .clone()
            .or_else(|| account.and_then(|account| account.login.clone()))
            .unwrap_or_else(|| "—".into());
        let login = account
            .and_then(|account| account.login.clone())
            .unwrap_or_else(|| "—".into());

        let account_card = card(theme)
            .child(section_title(theme, "Аккаунт"))
            .child(info_row(theme, "Логин", login))
            .child(info_row(theme, "UID", uid))
            .child(info_row(theme, "Регион", region))
            .child(info_row(theme, "Дата регистрации", registered))
            .child(info_row(theme, "Email", email));

        let subscription_card = card(theme)
            .child(section_title(theme, "Подписка"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(space::MD))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(0.0))
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(text::XL))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(plan),
                            )
                            .child(
                                div()
                                    .text_size(px(text::SM))
                                    .text_color(theme.colors.text_muted)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(if has_plus {
                                        "Слушайте музыку без рекламы и ограничений"
                                    } else {
                                        "Оформите Plus, чтобы слушать без ограничений"
                                    }),
                            ),
                    )
                    .child(
                        Badge::new(if has_plus {
                            "Активна"
                        } else {
                            "Не активна"
                        })
                        .tone(if has_plus {
                            BadgeTone::Success
                        } else {
                            BadgeTone::Outline
                        }),
                    ),
            )
            .child(
                div()
                    .pt(px(space::SM))
                    .child(info_row(theme, "Действует до", until))
                    .child(info_row(
                        theme,
                        "Пробный период",
                        if status.pretrial_active.unwrap_or(false) {
                            "Активен".to_string()
                        } else {
                            "Не активен".to_string()
                        },
                    ))
                    .child(info_row(
                        theme,
                        "Права",
                        status
                            .permissions
                            .as_ref()
                            .map(|permissions| {
                                if permissions.values.is_empty() {
                                    "Базовые".to_string()
                                } else {
                                    permissions.values.join(", ")
                                }
                            })
                            .unwrap_or_else(|| "—".into()),
                    ))
                    .child(info_row(
                        theme,
                        "Управление",
                        "В приложении Яндекса".to_string(),
                    )),
            );

        if self.viewport_width < 1120.0 {
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(space::LG))
                .child(account_card)
                .child(subscription_card)
                .into_any_element()
        } else {
            div()
                .w_full()
                .flex()
                .gap(px(space::LG))
                .child(div().flex_1().min_w(px(0.0)).child(account_card))
                .child(div().flex_1().min_w(px(0.0)).child(subscription_card))
                .into_any_element()
        }
    }

    fn liked_section(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(profile) = &self.profile else {
            return div().into_any_element();
        };

        let mut section = card(theme).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(space::MD))
                        .child(section_title(theme, "Любимые треки"))
                        .child(
                            Badge::new(format::plural(
                                profile.total_liked as u64,
                                "трек",
                                "трека",
                                "треков",
                            ))
                            .tone(BadgeTone::Neutral),
                        ),
                )
                .child(
                    div().flex().items_center().gap(px(space::SM)).child(
                        Button::new("shuffle-liked", "Перемешать")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Small)
                            .icon(icons::SHUFFLE)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.play_random(cx);
                            })),
                    ),
                ),
        );

        if profile.sample.is_empty() {
            section = section.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(space::MD))
                    .py(px(space::XXL))
                    .child(
                        div()
                            .size(px(52.0))
                            .rounded(px(16.0))
                            .bg(theme.colors.surface_active)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(icons::HEART)
                                    .size(24.0)
                                    .color(theme.colors.text_muted),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(text::LG))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Здесь пока пусто"),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .text_center()
                            .max_w(px(360.0))
                            .child(
                                "Поставьте «Мне нравится» нескольким трекам в Яндекс Музыке и обновите профиль.",
                            ),
                    )
                    .child(
                        Button::new("refresh-empty", "Обновить профиль")
                            .variant(ButtonVariant::Secondary)
                            .icon(icons::REFRESH)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.retry_profile(cx);
                            })),
                    ),
            );
        } else {
            let current_id = self.player.track.as_ref().map(|track| track.id.clone());
            let playing = self.player.playing;

            for (index, track) in profile.sample.iter().enumerate() {
                let is_current = current_id.as_deref() == Some(track.id.as_str());
                let cover = self.covers.get(&track.id).cloned();
                let row = TrackRow::new(
                    ElementId::Name(SharedString::from(format!("liked-{}-{index}", track.id))),
                    index + 1,
                    track.display_title().to_string(),
                    track.artist_names(),
                    format::duration_or_dash(track.duration()),
                )
                .cover(cover)
                .current(is_current, playing)
                .on_play(cx.listener({
                    let track = track.clone();
                    move |this, _: &gpui::ClickEvent, _, cx| {
                        this.play_track(track.clone(), cx);
                    }
                }));

                section = section.child(row);
            }

            if profile.total_liked > profile.sample.len() {
                section = section.child(
                    div()
                        .pt(px(space::SM))
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child(format!(
                            "Показано {} из {} треков",
                            profile.sample.len(),
                            profile.total_liked
                        )),
                );
            }
        }

        section.into_any_element()
    }

    fn search_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let results = self.search.results.clone();
        let artists = self.search.artists.clone();
        let albums = self.search.albums.clone();
        let kind = self.search.kind;
        let total = self.search.total;
        let loading = self.search.loading;
        let searched = self.search.searched;
        let query = self.search.query.clone();
        let page = self.search.page;
        let per_page = self.search.per_page.max(1);
        let pages = ((total as u64).saturating_add(per_page - 1) / per_page).max(1);
        let error = self.search.error.clone();
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;

        let shown = match kind {
            SearchKind::Tracks => results.len(),
            SearchKind::Artists => artists.len(),
            SearchKind::Albums => albums.len(),
        };
        let (result_title, noun_one, noun_few, noun_many) = match kind {
            SearchKind::Tracks => ("Результаты", "трек", "трека", "треков"),
            SearchKind::Artists => ("Исполнители", "исполнитель", "исполнителя", "исполнителей"),
            SearchKind::Albums => ("Альбомы", "альбом", "альбома", "альбомов"),
        };

        let header = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .text_size(px(text::DISPLAY))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("Поиск"),
            )
            .child(
                div()
                    .text_size(px(text::SM))
                    .text_color(theme.colors.text_secondary)
                    .child("Треки, исполнители и альбомы по всему каталогу"),
            );

        let tab = |id: &'static str,
                   label: &'static str,
                   selected: bool,
                   kind: SearchKind,
                   cx: &mut Context<Self>| {
            Button::new(id, label)
                .variant(if selected {
                    ButtonVariant::Soft
                } else {
                    ButtonVariant::Ghost
                })
                .size(ButtonSize::Small)
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                    this.set_search_kind(kind, cx);
                }))
        };

        let mut search_card = card(theme)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(self.search_input.clone()),
                    )
                    .child(
                        Button::new("search-submit", "Найти")
                            .variant(ButtonVariant::Primary)
                            .icon(icons::SEARCH)
                            .loading(loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.perform_search(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::XS))
                    .pt(px(space::XS))
                    .child(tab(
                        "search-tab-tracks",
                        "Треки",
                        kind == SearchKind::Tracks,
                        SearchKind::Tracks,
                        cx,
                    ))
                    .child(tab(
                        "search-tab-artists",
                        "Исполнители",
                        kind == SearchKind::Artists,
                        SearchKind::Artists,
                        cx,
                    ))
                    .child(tab(
                        "search-tab-albums",
                        "Альбомы",
                        kind == SearchKind::Albums,
                        SearchKind::Albums,
                        cx,
                    )),
            );

        if self.demo {
            search_card = search_card.child(
                div()
                    .text_size(px(text::XS))
                    .text_color(theme.colors.text_muted)
                    .child("В демо-режиме поиск работает по локальной коллекции."),
            );
        }

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));

        if loading {
            body = body.child(
                card(theme)
                    .items_center()
                    .py(px(space::XXL))
                    .child(Spinner::new().size(24.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child(format!("Ищем «{query}»…")),
                    ),
            );
        } else if !searched {
            let (title, detail) = match kind {
                SearchKind::Tracks => (
                    "Что будем слушать?",
                    "Введите название трека, исполнителя или альбома — и нажмите «Найти».",
                ),
                SearchKind::Artists => (
                    "Найдите исполнителя",
                    "Введите имя исполнителя, чтобы увидеть его в каталоге.",
                ),
                SearchKind::Albums => (
                    "Найдите альбом",
                    "Введите название альбома или исполнителя.",
                ),
            };
            body = body.child(empty_search_state(theme, icons::SEARCH, title, detail));
        } else {
            if let Some(error) = error {
                body = body.child(error_banner(theme, &error));
            } else if shown == 0 {
                body = body.child(empty_search_state(
                    theme,
                    icons::SEARCH,
                    "Ничего не найдено",
                    &format!(
                        "По запросу «{query}» ничего не нашлось. Попробуйте сформулировать иначе."
                    ),
                ));
            } else {
                let mut list = card(theme).child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(space::MD))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(space::MD))
                                .child(section_title(theme, result_title))
                                .child(
                                    Badge::new(format::plural(
                                        total as u64,
                                        noun_one,
                                        noun_few,
                                        noun_many,
                                    ))
                                    .tone(BadgeTone::Neutral),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child(if total > shown {
                                    format!("Показано {shown} из {total}")
                                } else {
                                    String::new()
                                }),
                        ),
                );

                match kind {
                    SearchKind::Tracks => {
                        for (index, track) in results.iter().enumerate() {
                            let is_current = current_id.as_deref() == Some(track.id.as_str());
                            let cover = self.covers.get(&track.id).cloned();
                            let row = TrackRow::new(
                                ElementId::Name(SharedString::from(format!(
                                    "search-{}-{index}",
                                    track.id
                                ))),
                                index + 1,
                                track.display_title().to_string(),
                                track.artist_names(),
                                format::duration_or_dash(track.duration()),
                            )
                            .cover(cover)
                            .current(is_current, playing)
                            .on_play(cx.listener({
                                let track = track.clone();
                                move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.play_search_result(track.clone(), cx);
                                }
                            }));

                            list = list.child(row);
                        }
                    }
                    SearchKind::Artists => {
                        for (index, artist) in artists.iter().enumerate() {
                            let id = artist.id.unwrap_or(index as u64);
                            let image = self.search_images.get(&format!("artist-{id}")).cloned();
                            let name = artist.display_name().to_string();
                            let genres = if artist.genres.is_empty() {
                                "Исполнитель".to_string()
                            } else {
                                artist.genres.join(", ")
                            };

                            let mut row = div()
                                .id(ElementId::Name(SharedString::from(format!(
                                    "search-artist-{id}-{index}"
                                ))))
                                .flex()
                                .items_center()
                                .gap(px(space::MD))
                                .h(px(60.0))
                                .px(px(space::SM))
                                .rounded(px(10.0))
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.colors.surface_hover))
                                .on_click(cx.listener({
                                    let name = name.clone();
                                    move |this, _: &gpui::ClickEvent, _, cx| {
                                        this.search_for_artist(name.clone(), cx);
                                    }
                                }))
                                .child(search_thumb(theme, image, icons::USER, 44.0, 10.0))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .gap(px(1.0))
                                        .child(
                                            div()
                                                .text_size(px(text::BASE))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(name),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(text::XS))
                                                .text_color(theme.colors.text_muted)
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(genres),
                                        ),
                                )
                                .child(
                                    Icon::new(icons::CHEVRON_RIGHT)
                                        .size(15.0)
                                        .color(theme.colors.text_muted),
                                );

                            row = row.w_full();
                            list = list.child(row);
                        }
                    }
                    SearchKind::Albums => {
                        for (index, album) in albums.iter().enumerate() {
                            let id = album.id.unwrap_or(index as u64);
                            let image = self.search_images.get(&format!("album-{id}")).cloned();
                            let title = album.display_title().to_string();
                            let artists = album.artist_names();
                            let meta = match (artists.is_empty(), album.year) {
                                (false, Some(year)) => format!("{artists} · {year}"),
                                (false, None) => artists,
                                (true, Some(year)) => year.to_string(),
                                (true, None) => "Альбом".to_string(),
                            };
                            let album_for_click = album.clone();

                            let row = div()
                                .id(ElementId::Name(SharedString::from(format!(
                                    "search-album-{id}-{index}"
                                ))))
                                .flex()
                                .items_center()
                                .gap(px(space::MD))
                                .h(px(60.0))
                                .px(px(space::SM))
                                .rounded(px(10.0))
                                .cursor_pointer()
                                .hover(|style| style.bg(theme.colors.surface_hover))
                                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                                    this.search_for_album(album_for_click.clone(), cx);
                                }))
                                .child(search_thumb(theme, image, icons::MUSIC, 44.0, 6.0))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .gap(px(1.0))
                                        .child(
                                            div()
                                                .text_size(px(text::BASE))
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(title),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(text::XS))
                                                .text_color(theme.colors.text_muted)
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .text_ellipsis()
                                                .child(meta),
                                        ),
                                )
                                .child(
                                    Icon::new(icons::CHEVRON_RIGHT)
                                        .size(15.0)
                                        .color(theme.colors.text_muted),
                                );

                            list = list.child(row);
                        }
                    }
                }

                if !self.demo && total > shown {
                    list = list.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(space::SM))
                            .pt(px(space::MD))
                            .child(
                                Button::new("search-prev", "Назад")
                                    .variant(ButtonVariant::Secondary)
                                    .size(ButtonSize::Small)
                                    .icon(icons::CHEVRON_LEFT)
                                    .disabled(page == 0 || loading)
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.search_previous(cx);
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(text::XS))
                                    .text_color(theme.colors.text_muted)
                                    .child(format!("Страница {} из {pages}", page + 1)),
                            )
                            .child(
                                Button::new("search-next", "Вперёд")
                                    .variant(ButtonVariant::Secondary)
                                    .size(ButtonSize::Small)
                                    .trailing_icon(icons::CHEVRON_RIGHT)
                                    .disabled(
                                        (page as u64 + 1) * per_page >= total as u64 || loading,
                                    )
                                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                        this.search_next(cx);
                                    })),
                            ),
                    );
                }

                body = body.child(list);
            }
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .id("search-scroll")
            .overflow_y_scroll()
            .child(
                div().w_full().flex().justify_center().child(
                    div()
                        .w_full()
                        .max_w(px(1180.0))
                        .p(px(space::XL))
                        .flex()
                        .flex_col()
                        .gap(px(space::LG))
                        .child(header)
                        .child(search_card)
                        .child(body)
                        .child(
                            div()
                                .pt(px(space::SM))
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child("Поиск выполняется через официальный API Яндекс Музыки."),
                        ),
                ),
            )
            .into_any_element()
    }

    fn playlists_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let main = self.playlists_content(theme, cx);
        self.app_shell(theme, cx, main)
    }

    fn playlist_detail_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let main = self.playlist_detail_content(theme, cx);
        self.app_shell(theme, cx, main)
    }

    fn playlists_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let items = self.playlists.items.clone();
        let loading = self.playlists.loading;
        let loaded = self.playlists.loaded;
        let error = self.playlists.error.clone();

        let header = div()
            .flex()
            .items_end()
            .justify_between()
            .gap(px(space::LG))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(text::DISPLAY))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Плейлисты"),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Ваши коллекции Яндекс Музыки"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .when(!items.is_empty(), |element| {
                        element.child(
                            Badge::new(format::plural(
                                items.len() as u64,
                                "плейлист",
                                "плейлиста",
                                "плейлистов",
                            ))
                            .tone(BadgeTone::Neutral),
                        )
                    })
                    .child(
                        IconButton::new("refresh-playlists", icons::REFRESH)
                            .size(38.0)
                            .icon_size(16.0)
                            .variant(ButtonVariant::Secondary)
                            .disabled(loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.load_playlists(cx);
                            })),
                    ),
            );

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));

        if let Some(error) = error {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .child(error_banner(theme, &error))
                    .child(
                        div().flex().justify_center().child(
                            Button::new("retry-playlists", "Повторить")
                                .variant(ButtonVariant::Secondary)
                                .icon(icons::REFRESH)
                                .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                    this.load_playlists(cx);
                                })),
                        ),
                    ),
            );
        }

        if loading && items.is_empty() {
            body = body.child(
                card(theme)
                    .items_center()
                    .py(px(space::XXL))
                    .child(Spinner::new().size(24.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Загружаем плейлисты…"),
                    ),
            );
        } else if !loaded && items.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::PLAYLIST,
                "Плейлисты не загружены",
                "Обновите список, чтобы увидеть ваши коллекции.",
            ));
        } else if items.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::PLAYLIST,
                "Пока нет плейлистов",
                "Создайте плейлист в Яндекс Музыке — он появится здесь.",
            ));
        } else {
            let columns = if self.viewport_width < 1100.0 { 2 } else { 3 };
            let mut grid = div().w_full().grid().grid_cols(columns).gap(px(space::LG));

            for playlist in &items {
                let id = playlist.playlist_id();
                let cover = self.playlist_cover_image(playlist);
                let title = playlist.display_title().to_string();
                let count = playlist.track_count.unwrap_or(playlist.tracks.len() as u64);
                let visibility = if playlist.is_public() {
                    "Публичный"
                } else {
                    "Приватный"
                };
                let playlist_for_click = playlist.clone();

                let card = div()
                    .id(ElementId::Name(SharedString::from(format!(
                        "playlist-card-{id}"
                    ))))
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .p(px(space::SM))
                    .rounded(px(14.0))
                    .bg(theme.colors.surface)
                    .border_1()
                    .border_color(theme.colors.border)
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(theme.colors.surface_hover)
                            .border_color(theme.colors.accent)
                    })
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                        this.open_playlist(playlist_for_click.clone(), cx);
                    }))
                    .child(playlist_cover(theme, cover, 150.0))
                    .child(
                        div()
                            .px(px(space::XS))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(text::BASE))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(title),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(space::SM))
                                    .text_size(px(text::XS))
                                    .text_color(theme.colors.text_muted)
                                    .child(format::plural(count, "трек", "трека", "треков"))
                                    .child(visibility.to_string()),
                            ),
                    );

                grid = grid.child(card);
            }

            body = body.child(grid);
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .id("playlists-scroll")
            .overflow_y_scroll()
            .child(
                div().w_full().flex().justify_center().child(
                    div()
                        .w_full()
                        .max_w(px(1180.0))
                        .p(px(space::XL))
                        .flex()
                        .flex_col()
                        .gap(px(space::LG))
                        .child(header)
                        .child(body)
                        .child(
                            div()
                                .pt(px(space::SM))
                                .text_size(px(text::XS))
                                .text_color(theme.colors.text_muted)
                                .child("Плейлисты загружаются из вашей медиатеки Яндекс Музыки."),
                        ),
                ),
            )
            .into_any_element()
    }

    fn playlist_detail_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(playlist) = self.playlists.selected.clone() else {
            return div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child("Плейлист не выбран")
                .into_any_element();
        };

        let cover = self.playlist_cover_image(&playlist);
        let title = playlist.display_title().to_string();
        let owner = playlist
            .owner
            .as_ref()
            .and_then(|owner| {
                owner
                    .display_name
                    .as_deref()
                    .or(owner.full_name.as_deref())
                    .or(owner.login.as_deref())
            })
            .unwrap_or("Яндекс Музыка")
            .to_string();
        let is_public = playlist.is_public();
        let total = playlist
            .track_count
            .unwrap_or(self.playlists.tracks.len() as u64);
        let duration = playlist.duration();
        let tracks = self.playlists.tracks.clone();
        let loading = self.playlists.tracks_loading;
        let error = self.playlists.tracks_error.clone();
        let current_id = self.player.track.as_ref().map(|track| track.id.clone());
        let playing = self.player.playing;

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                Button::new("back-to-playlists", "К плейлистам")
                    .variant(ButtonVariant::Ghost)
                    .icon(icons::CHEVRON_LEFT)
                    .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                        this.back_to_playlists(cx);
                    })),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(space::SM))
                    .child(
                        Button::new("play-playlist", "Слушать")
                            .variant(ButtonVariant::Primary)
                            .icon(icons::PLAY)
                            .disabled(tracks.is_empty() || loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.play_playlist(cx);
                            })),
                    )
                    .child(
                        Button::new("shuffle-playlist", "Перемешать")
                            .variant(ButtonVariant::Secondary)
                            .icon(icons::SHUFFLE)
                            .disabled(tracks.is_empty() || loading)
                            .on_click(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                                this.shuffle_playlist(cx);
                            })),
                    ),
            );

        let hero = div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(space::XL))
            .p(px(space::XL))
            .rounded(px(20.0))
            .bg(theme.colors.surface)
            .border_1()
            .border_color(theme.colors.border)
            .child(playlist_cover(theme, cover, 168.0).w(px(168.0)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(space::SM))
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .child(
                                div()
                                    .text_size(px(30.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .line_height(px(34.0))
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(title),
                            )
                            .child(
                                Badge::new(if is_public {
                                    "Публичный"
                                } else {
                                    "Приватный"
                                })
                                .tone(if is_public {
                                    BadgeTone::Outline
                                } else {
                                    BadgeTone::Neutral
                                }),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child(owner),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::XL))
                            .pt(px(space::SM))
                            .child(
                                Stat::new(
                                    format::plural(total, "трек", "трека", "треков"),
                                    "в плейлисте",
                                )
                                .accent(true),
                            )
                            .child(divider_vertical(theme))
                            .child(Stat::new(format::duration(duration), "длительность")),
                    )
                    .when(
                        playlist
                            .description
                            .as_deref()
                            .is_some_and(|description| !description.is_empty()),
                        |element| {
                            element.child(
                                div()
                                    .text_size(px(text::SM))
                                    .text_color(theme.colors.text_muted)
                                    .max_w(px(560.0))
                                    .child(playlist.description.clone().unwrap_or_default()),
                            )
                        },
                    ),
            );

        let mut body = div().w_full().flex().flex_col().gap(px(space::LG));

        if let Some(error) = error {
            body = body.child(error_banner(theme, &error));
        }

        if loading {
            body = body.child(
                card(theme)
                    .items_center()
                    .py(px(space::XXL))
                    .child(Spinner::new().size(24.0).color(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(text::SM))
                            .text_color(theme.colors.text_secondary)
                            .child("Загружаем треки плейлиста…"),
                    ),
            );
        } else if tracks.is_empty() {
            body = body.child(empty_search_state(
                theme,
                icons::PLAYLIST,
                "Плейлист пуст",
                "Добавьте треки в плейлист в Яндекс Музыке и обновите страницу.",
            ));
        } else {
            let mut list = card(theme).child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(space::MD))
                            .child(section_title(theme, "Треки")),
                    )
                    .child(
                        Badge::new(format::plural(total, "трек", "трека", "треков"))
                            .tone(BadgeTone::Neutral),
                    ),
            );

            for (index, track) in tracks.iter().enumerate() {
                let is_current = current_id.as_deref() == Some(track.id.as_str());
                let track_cover = self.covers.get(&track.id).cloned();
                let row = TrackRow::new(
                    ElementId::Name(SharedString::from(format!(
                        "playlist-track-{}-{index}",
                        track.id
                    ))),
                    index + 1,
                    track.display_title().to_string(),
                    track.artist_names(),
                    format::duration_or_dash(track.duration()),
                )
                .cover(track_cover)
                .current(is_current, playing)
                .on_play(cx.listener({
                    let track = track.clone();
                    move |this, _: &gpui::ClickEvent, _, cx| {
                        this.play_playlist_track(track.clone(), cx);
                    }
                }));

                list = list.child(row);
            }

            if total as usize > tracks.len() {
                list = list.child(
                    div()
                        .pt(px(space::SM))
                        .text_size(px(text::XS))
                        .text_color(theme.colors.text_muted)
                        .text_center()
                        .child(format!("Показано {} из {} треков", tracks.len(), total)),
                );
            }

            body = body.child(list);
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .id("playlist-detail-scroll")
            .overflow_y_scroll()
            .child(
                div().w_full().flex().justify_center().child(
                    div()
                        .w_full()
                        .max_w(px(1180.0))
                        .p(px(space::XL))
                        .flex()
                        .flex_col()
                        .gap(px(space::LG))
                        .child(header)
                        .child(hero)
                        .child(body),
                ),
            )
            .into_any_element()
    }

    fn playlist_cover_image(&self, playlist: &Playlist) -> Option<Arc<Image>> {
        self.playlist_covers
            .get(&playlist.playlist_id())
            .cloned()
            .or_else(|| {
                playlist
                    .tracks
                    .first()
                    .and_then(|short| self.covers.get(&short.id).cloned())
            })
    }

    fn player_bar(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
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
        let elapsed = format::duration(self.player.position);

        let view = cx.entity();
        let volume_view = cx.entity();
        let bar = PlayerBar::new()
            .track(title, artist)
            .cover(self.player.cover.clone())
            .progress(self.player.progress(), elapsed, duration)
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
            .on_seek(move |fraction, _, cx| {
                view.update(cx, |this, cx| this.seek_to(fraction, cx));
            });

        let _ = theme;
        bar.into_any_element()
    }

    fn account(&self) -> Option<&yandex_music::Account> {
        self.profile
            .as_ref()
            .and_then(|profile| profile.status.account.as_ref())
    }
}

fn empty_search_state(theme: &Theme, icon: &'static str, title: &str, detail: &str) -> Div {
    card(theme)
        .items_center()
        .py(px(space::XXL))
        .child(
            div()
                .size(px(56.0))
                .rounded(px(18.0))
                .bg(theme.colors.surface_active)
                .flex()
                .items_center()
                .justify_center()
                .child(Icon::new(icon).size(26.0).color(theme.colors.text_muted)),
        )
        .child(
            div()
                .text_size(px(text::LG))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(title.to_string()),
        )
        .child(
            div()
                .text_size(px(text::SM))
                .text_color(theme.colors.text_secondary)
                .text_center()
                .max_w(px(420.0))
                .child(detail.to_string()),
        )
}

fn playlist_cover(theme: &Theme, image: Option<Arc<Image>>, height: f32) -> Div {
    let content = match image {
        Some(image) => img(ImageSource::Image(image))
            .size_full()
            .object_fit(ObjectFit::Cover)
            .into_any_element(),
        None => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.accent_soft)
            .child(
                Icon::new(icons::PLAYLIST)
                    .size(36.0)
                    .color(theme.colors.accent),
            )
            .into_any_element(),
    };

    div()
        .w_full()
        .h(px(height))
        .rounded(px(12.0))
        .overflow_hidden()
        .bg(theme.colors.surface_active)
        .flex_shrink_0()
        .child(content)
}

fn search_thumb(
    theme: &Theme,
    image: Option<Arc<Image>>,
    fallback_icon: &'static str,
    size: f32,
    radius: f32,
) -> Div {
    let content = match image {
        Some(image) => img(ImageSource::Image(image))
            .size_full()
            .object_fit(ObjectFit::Cover)
            .into_any_element(),
        None => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.surface_active)
            .child(
                Icon::new(fallback_icon)
                    .size(size * 0.42)
                    .color(theme.colors.text_muted),
            )
            .into_any_element(),
    };

    div()
        .size(px(size))
        .rounded(px(radius))
        .overflow_hidden()
        .flex_shrink_0()
        .child(content)
}

fn screen_title(screen: &Screen) -> &'static str {
    match screen {
        Screen::Login => "Вход",
        Screen::Loading => "Загрузка",
        Screen::Profile => "Профиль",
        Screen::Search => "Поиск",
        Screen::Playlists | Screen::Playlist => "Плейлисты",
        Screen::LoadError(_) => "Ошибка",
    }
}

/// A slim client-side titlebar with a draggable area and window controls.
fn title_bar(screen: &Screen, window: &mut Window, theme: &Theme) -> AnyElement {
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

fn window_control(
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

fn resize_handle(id: &'static str, edge: ResizeEdge, cursor: CursorStyle) -> Stateful<Div> {
    div()
        .id(id)
        .cursor(cursor)
        .on_mouse_down(MouseButton::Left, move |_, window, _| {
            window.start_window_resize(edge);
        })
}

// ------------------------------------------------------------------ Helpers

fn brand(theme: &Theme, large: bool) -> Div {
    let mark_size = if large { 44.0 } else { 36.0 };
    let title_size = if large { text::XL } else { text::LG };

    div()
        .flex()
        .items_center()
        .gap(px(space::MD))
        .child(
            div()
                .size(px(mark_size))
                .rounded(px(mark_size * 0.3))
                .bg(theme.colors.accent)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(icons::MUSIC)
                        .size(mark_size * 0.5)
                        .color(theme.colors.accent_text),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(
                    div()
                        .text_size(px(title_size))
                        .font_weight(gpui::FontWeight::BOLD)
                        .line_height(px(title_size + 2.0))
                        .child("rsyameow"),
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(theme.colors.text_muted)
                        .child("YANDEX MUSIC CLIENT"),
                ),
        )
}

fn blob(color: gpui::Rgba, x: f32, y: f32, size: f32) -> Div {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .size(px(size))
        .rounded_full()
        .bg(color)
}

fn divider(theme: &Theme, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(space::MD))
        .child(div().flex_1().h(px(1.0)).bg(theme.colors.border))
        .child(
            div()
                .text_size(px(text::XS))
                .text_color(theme.colors.text_muted)
                .child(label.to_string()),
        )
        .child(div().flex_1().h(px(1.0)).bg(theme.colors.border))
}

fn divider_vertical(theme: &Theme) -> Div {
    div()
        .w(px(1.0))
        .h(px(34.0))
        .bg(theme.colors.border)
        .flex_shrink_0()
}

fn error_banner(theme: &Theme, message: &str) -> Div {
    div()
        .w_full()
        .flex()
        .items_start()
        .gap(px(space::SM))
        .rounded(px(10.0))
        .bg(theme.colors.danger_soft)
        .border_1()
        .border_color(theme.colors.danger)
        .p(px(space::MD))
        .child(
            Icon::new(icons::WARNING)
                .size(16.0)
                .color(theme.colors.danger),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(text::SM))
                .text_color(theme.colors.danger)
                .child(message.to_string()),
        )
}

fn section_label(theme: &Theme, label: &str) -> Div {
    div()
        .px(px(space::MD))
        .text_size(px(10.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.colors.text_muted)
        .child(label.to_string())
}

fn section_title(theme: &Theme, label: &str) -> Div {
    div()
        .text_size(px(text::LG))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(theme.colors.text)
        .child(label.to_string())
}

fn card(theme: &Theme) -> Div {
    div()
        .w_full()
        .rounded(px(16.0))
        .bg(theme.colors.surface)
        .border_1()
        .border_color(theme.colors.border)
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
}

fn info_row(theme: &Theme, label: &str, value: String) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(space::MD))
        .py(px(6.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .text_size(px(text::SM))
                .text_color(theme.colors.text_muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(text::SM))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.colors.text)
                .max_w(px(280.0))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(value),
        )
}
