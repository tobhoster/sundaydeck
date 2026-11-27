use gpui::{
    App, ClipboardItem, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::actions::{EditCopy, EditCut, EditPaste, EditSelectAll};
use crate::design::tokens::{FONT_BODY, RADIUS_CONTROL, TOKENS};

#[derive(Debug, Clone)]
pub enum TextInputEvent {
    Changed(String),
    Submitted(String),
}

/// Reusable keyboard-complete GPUI text input supporting single-line and multi-line editing,
/// cursor motion, clipboard copy/paste, and isolated `TextInput` key context so typing never
/// fires presentation hotkeys like `Space` or arrow keys.
pub struct TextInput {
    id: String,
    value: String,
    placeholder: String,
    multiline: bool,
    cursor_byte: usize,
    focus_handle: FocusHandle,
}

impl EventEmitter<TextInputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TextInput {
    pub fn single_line(
        id: impl Into<String>,
        placeholder: impl Into<String>,
        initial: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = initial.into();
        let cursor_byte = value.len();
        Self {
            id: id.into(),
            value,
            placeholder: placeholder.into(),
            multiline: false,
            cursor_byte,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn multiline(
        id: impl Into<String>,
        placeholder: impl Into<String>,
        initial: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = initial.into();
        let cursor_byte = value.len();
        Self {
            id: id.into(),
            value,
            placeholder: placeholder.into(),
            multiline: true,
            cursor_byte,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.value = value.into();
        self.cursor_byte = self.value.len();
        cx.notify();
    }

    fn on_mouse_down(&mut self, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        self.cursor_byte = self.value.len();
        cx.notify();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &event.keystroke;
        let cmd_or_ctrl = ks.modifiers.platform || ks.modifiers.control;

        if cmd_or_ctrl {
            match ks.key.as_str() {
                "v" => {
                    if let Some(item) = cx.read_from_clipboard()
                        && let Some(text) = item.text()
                    {
                        self.insert_str(&text, cx);
                    }
                    return;
                }
                "c" => {
                    cx.write_to_clipboard(ClipboardItem::new_string(self.value.clone()));
                    return;
                }
                "backspace" | "u" => {
                    self.value.clear();
                    self.cursor_byte = 0;
                    self.emit_changed(cx);
                    return;
                }
                "enter" if self.multiline => {
                    cx.emit(TextInputEvent::Submitted(self.value.clone()));
                    return;
                }
                _ => return,
            }
        }

        match ks.key.as_str() {
            "backspace" => self.delete_prev_char(cx),
            "delete" => self.delete_next_char(cx),
            "left" => self.move_left(cx),
            "right" => self.move_right(cx),
            "home" => {
                self.cursor_byte = 0;
                cx.notify();
            }
            "end" => {
                self.cursor_byte = self.value.len();
                cx.notify();
            }
            "enter" => {
                if self.multiline {
                    self.insert_str("\n", cx);
                } else {
                    cx.emit(TextInputEvent::Submitted(self.value.clone()));
                }
            }
            "space" => self.insert_str(" ", cx),
            _ => {
                if let Some(ch) = &ks.key_char
                    && !ch.chars().any(char::is_control)
                {
                    self.insert_str(ch, cx);
                }
            }
        }
    }

    fn insert_str(&mut self, text: &str, cx: &mut Context<Self>) {
        let filtered: String = if self.multiline {
            text.replace("\r\n", "\n")
        } else {
            text.replace(['\r', '\n'], " ")
        };
        if filtered.is_empty() {
            return;
        }
        let pos = self.cursor_byte.min(self.value.len());
        self.value.insert_str(pos, &filtered);
        self.cursor_byte = pos + filtered.len();
        self.emit_changed(cx);
    }

    fn delete_prev_char(&mut self, cx: &mut Context<Self>) {
        if self.cursor_byte == 0 || self.value.is_empty() {
            return;
        }
        let prev = prev_char_boundary(&self.value, self.cursor_byte);
        self.value.drain(prev..self.cursor_byte);
        self.cursor_byte = prev;
        self.emit_changed(cx);
    }

    fn delete_next_char(&mut self, cx: &mut Context<Self>) {
        if self.cursor_byte >= self.value.len() {
            return;
        }
        let next = next_char_boundary(&self.value, self.cursor_byte);
        self.value.drain(self.cursor_byte..next);
        self.emit_changed(cx);
    }

    fn move_left(&mut self, cx: &mut Context<Self>) {
        if self.cursor_byte > 0 {
            self.cursor_byte = prev_char_boundary(&self.value, self.cursor_byte);
            cx.notify();
        }
    }

    fn move_right(&mut self, cx: &mut Context<Self>) {
        if self.cursor_byte < self.value.len() {
            self.cursor_byte = next_char_boundary(&self.value, self.cursor_byte);
            cx.notify();
        }
    }

    fn emit_changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(TextInputEvent::Changed(self.value.clone()));
        cx.notify();
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        let is_empty = self.value.is_empty();

        let display_text = if is_empty {
            if focused {
                "|".to_owned()
            } else {
                self.placeholder.clone()
            }
        } else if focused {
            let pos = self.cursor_byte.min(self.value.len());
            format!("{}|{}", &self.value[..pos], &self.value[pos..])
        } else {
            self.value.clone()
        };

        let mut box_el = div()
            .id(self.id.clone())
            .key_context("TextInput")
            .track_focus(&self.focus_handle)
            .w_full()
            .px(px(9.0))
            .py(px(if self.multiline { 7.0 } else { 5.0 }))
            .rounded(px(RADIUS_CONTROL))
            .cursor_text()
            .bg(TOKENS.canvas)
            .border_1()
            .border_color(if focused {
                TOKENS.focus_border
            } else {
                TOKENS.gridline
            })
            .hover(|s| {
                if focused {
                    s
                } else {
                    s.border_color(TOKENS.focus_border)
                }
            })
            .font_family(FONT_BODY)
            .text_size(px(12.0))
            .line_height(px(17.0))
            .text_color(if is_empty && !focused {
                TOKENS.ink_muted
            } else if is_empty && focused {
                TOKENS.signal_preview
            } else {
                TOKENS.ink_primary
            })
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(|this, _: &EditCut, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(this.value.clone()));
                this.value.clear();
                this.cursor_byte = 0;
                this.emit_changed(cx);
            }))
            .on_action(cx.listener(|this, _: &EditCopy, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(this.value.clone()));
            }))
            .on_action(cx.listener(|this, _: &EditPaste, _, cx| {
                if let Some(item) = cx.read_from_clipboard()
                    && let Some(text) = item.text()
                {
                    this.insert_str(&text, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &EditSelectAll, _, cx| {
                this.cursor_byte = this.value.len();
                cx.notify();
            }));

        if self.multiline {
            box_el = box_el.min_h(px(96.0)).overflow_y_scroll();
        }

        box_el.child(display_text)
    }
}

fn prev_char_boundary(s: &str, idx: usize) -> usize {
    let mut i = idx.min(s.len()).saturating_sub(1);
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn next_char_boundary(s: &str, idx: usize) -> usize {
    let mut i = (idx + 1).min(s.len());
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}
