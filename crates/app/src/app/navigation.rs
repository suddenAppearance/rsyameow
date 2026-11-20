//! Top-level screen navigation and history.

use super::*;

impl RootView {
    pub(crate) fn open_profile(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.screen = Screen::Profile;
        self.nav_stack.clear();
        self.queue_open = false;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn open_liked(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.screen = Screen::Liked;
        self.nav_stack.clear();
        self.queue_open = false;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn push_navigation(&mut self) {
        if !matches!(
            self.screen,
            Screen::Login | Screen::Loading | Screen::LoadError(_)
        ) {
            self.nav_stack.push(self.screen.clone());
        }
    }

    pub(crate) fn navigate_back(&mut self, cx: &mut Context<Self>) {
        self.screen = self.nav_stack.pop().unwrap_or(Screen::Profile);
        self.queue_open = false;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn open_settings(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.screen = Screen::Settings;
        self.quality_select_open = false;
        self.eq_drag = None;
        self.nav_stack.clear();
        self.queue_open = false;
        self.schedule_state_save(cx);
        cx.notify();
    }

    pub(crate) fn open_now_playing(&mut self, cx: &mut Context<Self>) {
        if matches!(self.screen, Screen::Login | Screen::Loading) {
            return;
        }
        self.screen = Screen::NowPlaying;
        self.queue_open = false;
        self.lyrics_expanded = false;
        self.nav_stack.clear();
        if let Some(track) = self.player.track.clone() {
            self.load_covers(vec![track.clone()], cx);
            self.load_track_lyrics(track.clone(), cx);
            self.load_track_backdrop(track, cx);
        }
        self.schedule_state_save(cx);
        cx.notify();
    }
}
