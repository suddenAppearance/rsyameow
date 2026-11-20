//! Login, loading and fatal-error screens.

use super::widgets::*;
use super::*;

impl RootView {
    pub(crate) fn login_view(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
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

    pub(crate) fn loading_view(&mut self, theme: &Theme) -> AnyElement {
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

    pub(crate) fn load_error_view(
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
}
