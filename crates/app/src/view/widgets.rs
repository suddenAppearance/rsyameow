//! Shared cards, covers, dividers and background painting.

use super::*;

pub(crate) fn empty_search_state(
    theme: &Theme,
    icon: &'static str,
    title: &str,
    detail: &str,
) -> Div {
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

pub(crate) fn playlist_cover(theme: &Theme, image: Option<Arc<Image>>, height: f32) -> Div {
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

pub(crate) fn search_thumb(
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

pub(crate) fn track_navigation(
    row: TrackRow,
    track: &Track,
    cx: &mut Context<RootView>,
) -> TrackRow {
    let track_for_title = track.clone();
    let row = row.on_title(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
        this.open_track_from_screen(track_for_title.clone(), cx);
    }));

    if let Some(artist) = track.artists.first().cloned()
        && let Some(artist_id) = artist.id
    {
        let artist_for_click = artist.clone();
        row.on_artist(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
            this.open_artist(artist_id, Some(artist_for_click.clone()), cx);
        }))
    } else {
        row
    }
}

pub(crate) fn glass_card(theme: &Theme) -> Div {
    div()
        .w_full()
        .rounded(px(18.0))
        .bg(rgba(0x0808088C))
        .border_1()
        .border_color(rgba(0xFFFFFF29))
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .shadow_lg()
        .text_color(theme.colors.white)
}

pub(crate) fn immersive_scroll_page(
    palette: (Rgba, Rgba),
    backdrop: Option<Arc<Image>>,
    scroll_id: &'static str,
    content: AnyElement,
) -> AnyElement {
    div()
        .relative()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_hidden()
        .bg(palette.0)
        .child(immersive_background(palette, backdrop))
        .child(
            div()
                .relative()
                .flex()
                .flex_col()
                .flex_1()
                .min_h(px(0.0))
                .id(scroll_id)
                .overflow_y_scroll()
                .child(content),
        )
        .into_any_element()
}

pub(crate) fn immersive_background(
    palette: (Rgba, Rgba),
    backdrop: Option<Arc<Image>>,
) -> AnyElement {
    let (base, glow) = palette;
    let glow_hsla: Hsla = glow.into();

    let mut layer = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .size_full()
        .bg(base);

    if let Some(backdrop) = backdrop {
        layer = layer.child(
            img(ImageSource::Image(backdrop))
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .opacity(0.52),
        );
    }

    layer
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    paint_liquid(window, bounds, glow_hsla);
                },
            )
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full(),
        )
        .child(
            div()
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .size_full()
                .bg(rgba(0x05050538)),
        )
        .into_any_element()
}

pub(crate) fn paint_liquid(window: &mut Window, bounds: Bounds<gpui::Pixels>, glow: Hsla) {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let colors = [
        Hsla {
            h: glow.h,
            s: 0.9,
            l: 0.58,
            a: 1.0,
        },
        Hsla {
            h: (glow.h + 0.18) % 1.0,
            s: 0.86,
            l: 0.55,
            a: 1.0,
        },
        Hsla {
            h: (glow.h + 0.72) % 1.0,
            s: 0.8,
            l: 0.52,
            a: 1.0,
        },
        Hsla {
            h: (glow.h + 0.42) % 1.0,
            s: 0.74,
            l: 0.68,
            a: 1.0,
        },
    ];
    let placements = [
        (0.26, 0.3, 0.48),
        (0.74, 0.3, 0.44),
        (0.3, 0.72, 0.5),
        (0.76, 0.7, 0.4),
    ];

    let min_side = width.min(height);
    for (index, color) in colors.into_iter().enumerate() {
        let (x, y, radius_factor) = placements[index];
        let center_x = f32::from(bounds.origin.x) + width * x;
        let center_y = f32::from(bounds.origin.y) + height * y;
        let radius = min_side * radius_factor;
        let ring_alpha = 0.0045;

        for ring in (0..128).rev() {
            let progress = (ring + 1) as f32 / 128.0;
            let ring_radius = radius * progress;
            let blob = Bounds {
                origin: point(px(center_x - ring_radius), px(center_y - ring_radius)),
                size: size(px(ring_radius * 2.0), px(ring_radius * 2.0)),
            };
            window.paint_quad(quad(
                blob,
                px(ring_radius),
                Hsla {
                    a: ring_alpha,
                    ..color
                },
                px(0.0),
                gpui::transparent_black(),
                BorderStyle::default(),
            ));
        }
    }
}

// ------------------------------------------------------------------ Helpers

pub(crate) fn brand(theme: &Theme, large: bool) -> Div {
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
                    Icon::new(icons::PAW)
                        .size(mark_size * 0.58)
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
                        .child("Я.Мяу"),
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(theme.colors.text_muted)
                        .child("YANDEX MUSIC CLIENT"),
                ),
        )
}

pub(crate) fn blob(color: gpui::Rgba, x: f32, y: f32, size: f32) -> Div {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .size(px(size))
        .rounded_full()
        .bg(color)
}

pub(crate) fn divider(theme: &Theme, label: &str) -> Div {
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

pub(crate) fn divider_vertical(theme: &Theme) -> Div {
    div()
        .w(px(1.0))
        .h(px(34.0))
        .bg(theme.colors.border)
        .flex_shrink_0()
}

pub(crate) fn error_banner(theme: &Theme, message: &str) -> Div {
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

pub(crate) fn section_label(theme: &Theme, label: &str) -> Div {
    div()
        .px(px(space::MD))
        .text_size(px(10.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.colors.text_muted)
        .child(label.to_string())
}

pub(crate) fn section_title(theme: &Theme, label: &str) -> Div {
    div()
        .text_size(px(text::LG))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(theme.colors.text)
        .child(label.to_string())
}

pub(crate) fn card(theme: &Theme) -> Div {
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

pub(crate) fn info_row(theme: &Theme, label: &str, value: String) -> Div {
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
