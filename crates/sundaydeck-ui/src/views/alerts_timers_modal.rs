use gpui::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, Render, Subscription, Window,
};
use sundaydeck_core::{
    AlertPosition, AlertTimeout, AudienceAlert, PresentationCommand, ServiceTimer,
};

use crate::components::TextInput;
use crate::remote_control::build_parent_pager_alert;
use crate::store::PresentationStore;
use crate::views::alerts_timers_ui::render_alerts_timers_modal;
use crate::views::quick_edit_modal::ModalClosed;

/// Operator control modal for discreet Nursery/Parent-Paging Alerts (position + auto-dismiss)
/// and Pre-Service Countdown / Speaker Elapsed Timers.
pub struct AlertsTimersModal {
    pub(crate) store: Entity<PresentationStore>,
    pub(crate) parent_code_input: Entity<TextInput>,
    pub(crate) alert_input: Entity<TextInput>,
    pub(crate) alert_position: AlertPosition,
    pub(crate) alert_timeout: AlertTimeout,
    _store_sub: Subscription,
}

impl EventEmitter<ModalClosed> for AlertsTimersModal {}

impl AlertsTimersModal {
    pub fn new(store: Entity<PresentationStore>, cx: &mut Context<Self>) -> Self {
        let existing_msg = store
            .read(cx)
            .snapshot()
            .active_alert
            .as_ref()
            .map(|a| a.message.clone())
            .unwrap_or_else(|| "Nursery Alert: Parent #142 please check West Hall".to_owned());
        let existing_pos = store
            .read(cx)
            .snapshot()
            .active_alert
            .as_ref()
            .map_or(AlertPosition::BottomBar, |a| a.position);

        let parent_code_input = cx.new(|cx| {
            TextInput::single_line("parent-pager-code-input", "Code (e.g. 142, B-08)", "", cx)
        });
        let alert_input = cx.new(|cx| {
            TextInput::single_line(
                "alert-message-input",
                "Parent #142 — Nursery West",
                existing_msg,
                cx,
            )
        });
        let store_sub = cx.observe(&store, |_, _, cx| cx.notify());
        Self {
            store,
            parent_code_input,
            alert_input,
            alert_position: existing_pos,
            alert_timeout: AlertTimeout::Seconds { duration_secs: 30 },
            _store_sub: store_sub,
        }
    }

    /// 1-click nursery security-code pager: flashes a discreet corner badge (`Parent #142`)
    /// for 45 seconds without interrupting worship lyrics or sermon slides.
    pub(crate) fn send_parent_page(&mut self, cx: &mut Context<Self>) {
        let code = self.parent_code_input.read(cx).value().to_owned();
        let sent = self.store.update(cx, |s, cx| {
            let now = s.now_epoch_ms();
            let Some(alert) = build_parent_pager_alert(&code, now) else {
                return false;
            };
            let _ = s.dispatch(PresentationCommand::SetAudienceAlert(Some(alert)), cx);
            true
        });
        if sent {
            self.parent_code_input
                .update(cx, |inp, cx| inp.set_value("", cx));
        }
    }

    pub(crate) fn send_alert(&mut self, cx: &mut Context<Self>) {
        let msg = self.alert_input.read(cx).value().trim().to_owned();
        if msg.is_empty() {
            return;
        }
        let pos = self.alert_position;
        let timeout = self.alert_timeout;
        self.store.update(cx, |s, cx| {
            let now = s.now_epoch_ms();
            let alert = AudienceAlert::new(msg, pos, timeout, now);
            let _ = s.dispatch(PresentationCommand::SetAudienceAlert(Some(alert)), cx);
        });
    }

    pub(crate) fn start_preset_timer(&mut self, mut timer: ServiceTimer, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| {
            let now = s.now_epoch_ms();
            timer.start(now);
            let _ = s.dispatch(PresentationCommand::SetTimer(Some(timer)), cx);
        });
    }
}

impl Render for AlertsTimersModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_alerts_timers_modal(self, cx)
    }
}
