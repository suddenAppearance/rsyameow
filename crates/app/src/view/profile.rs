//! Profile and subscription rendering.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn profile_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let main = self.main_content(theme, cx);
        self.app_shell(theme, cx, main)
    }

    pub(crate) fn main_content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
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
                            .child("Аккаунт и подписка Яндекс Музыки"),
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
        let liked_summary = self.liked_summary(theme, cx);

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
                            .child(liked_summary)
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

    pub(crate) fn hero_card(
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

    pub(crate) fn details_row(&self, theme: &Theme, status: &Status) -> AnyElement {
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
}
