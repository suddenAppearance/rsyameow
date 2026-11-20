//! Rendering for [`RootView`].

use crate::app::{RootView, Screen, plan_name, plus_until, registered_year};
use crate::format;
use gpui::{
    AnyElement, Context, Div, ElementId, Focusable as _, IntoElement, Render, SharedString, Window,
    div, prelude::*, px, relative, rgba,
};
use rsyameow_ui::{
    Avatar, Badge, BadgeTone, Button, ButtonSize, ButtonVariant, Icon, IconButton, PlayerBar,
    Spinner, Stat, Theme, TrackRow, icons, space, text,
};
use yandex_music::Status;

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::get(cx).clone();
        let screen = self.screen.clone();

        let content: AnyElement = match screen {
            Screen::Login => self.login_view(&theme, cx),
            Screen::Loading => self.loading_view(&theme),
            Screen::LoadError(message) => self.load_error_view(&message, &theme, cx),
            Screen::Profile => self.profile_view(&theme, cx),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.background)
            .text_color(theme.colors.text)
            .font_family(theme.fonts.ui.clone())
            .child(content)
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

    fn profile_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(self.sidebar(theme, cx))
                    .child(self.main_content(theme, cx)),
            )
            .child(self.player_bar(theme, cx))
            .into_any_element()
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

        let nav_item = div()
            .id("nav-profile")
            .flex()
            .items_center()
            .gap(px(space::MD))
            .h(px(42.0))
            .px(px(space::MD))
            .rounded(px(10.0))
            .bg(theme.colors.accent_soft)
            .text_color(theme.colors.accent)
            .text_size(px(text::BASE))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child(Icon::new(icons::USER).size(17.0).color(theme.colors.accent))
            .child("Профиль");

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
                    .child(nav_item),
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

        div()
            .w_full()
            .flex()
            .gap(px(space::LG))
            .child(div().flex_1().min_w(px(0.0)).child(account_card))
            .child(div().flex_1().min_w(px(0.0)).child(subscription_card))
            .into_any_element()
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
                    format::duration(track.duration()),
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
            .map(|track| format::duration(track.duration()))
            .unwrap_or_else(|| "0:00".into());
        let elapsed = format::duration(self.player.position);

        let bar = PlayerBar::new()
            .track(title, artist)
            .cover(self.player.cover.clone())
            .progress(self.player.progress(), elapsed, duration)
            .playing(self.player.playing)
            .loading(self.player.loading)
            .quality(self.player.quality.clone())
            .error(self.player.error.clone())
            .on_toggle(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.toggle_playback(cx);
            }))
            .on_next(cx.listener(|this, _: &gpui::ClickEvent, _, cx| {
                this.play_random(cx);
            }));

        let _ = theme;
        bar.into_any_element()
    }

    fn account(&self) -> Option<&yandex_music::Account> {
        self.profile
            .as_ref()
            .and_then(|profile| profile.status.account.as_ref())
    }
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
