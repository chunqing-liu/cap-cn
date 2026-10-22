//! `SelectionHeader` -- the Done / "N … selected" / Delete row every segment
//! panel opens with.
//!
//! The source writes it out **eight times**, once per selectable track, with
//! only the noun and the delete action changing (`ConfigSidebar.tsx:1185-1211`
//! captions, `:1249-1275` keyboard, `:1321-1347` text, `:1394-1420` audio,
//! `:1461-1487` mask, `:1524-1573` zoom, `:1616-1642` 3D, `:1685-1710` scene,
//! plus the scene panel's own two-button copy at `:6326-6344`). One component
//! here, with the zoom panel's two extras -- the terser count and the inline
//! "Select all" -- as builder options, because that copy is the only one that
//! has them (`:1528-1537, 1553-1571`).
//!
//! Shape: `flex flex-row justify-between items-center`, a left group of
//! `flex gap-2 items-center` holding the Done button and a `text-sm
//! text-gray-10` count, and a right-hand danger Delete.

use gpui::{
    App, ClickEvent, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::theme::Theme;

use super::{ClickHandler, EditorButton};

/// `{count} {noun} {segment|segments} selected` -- the sentence seven of the
/// eight headers build inline.
pub fn selection_label(noun: &str, count: usize) -> String {
    let noun = match noun {
        "caption" => "字幕",
        "text" => "文字",
        "keyboard" => "按键",
        "audio" => "音频",
        "mask" => "遮罩",
        "scene" => "场景",
        other => other,
    };
    format!("已选择 {count} 个{noun}片段")
}

/// The zoom header's own, terser wording (`ConfigSidebar.tsx:1528-1537`): the
/// sidebar column is narrow, so the count drops the noun and gains a total.
pub fn zoom_selection_label(count: usize, total: usize) -> String {
    if total > 1 && count == total {
        format!("已全选 {total} 项")
    } else if total > 1 {
        format!("已选择 {count}/{total} 项")
    } else {
        format!("已选择 {count} 项")
    }
}

#[derive(IntoElement)]
pub struct SelectionHeader {
    id: SharedString,
    label: SharedString,
    /// The zoom header's inline text action, shown only while the selection is
    /// smaller than the track.
    select_all: Option<ClickHandler>,
    label_color: Hsla,
    action_color: Hsla,
    theme: Theme,
    on_done: Option<ClickHandler>,
    on_delete: Option<ClickHandler>,
}

impl SelectionHeader {
    pub fn plain(
        theme: &Theme,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            select_all: None,
            label_color: Hsla::from(theme.editor.text_2),
            action_color: Hsla::from(theme.editor.accent),
            theme: *theme,
            on_done: None,
            on_delete: None,
        }
    }

    pub fn on_done(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_done = Some(Box::new(handler));
        self
    }

    pub fn on_delete(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_delete = Some(Box::new(handler));
        self
    }

    pub fn on_select_all(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.select_all = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for SelectionHeader {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let SelectionHeader {
            id,
            label,
            select_all,
            label_color,
            action_color,
            theme,
            on_done,
            on_delete,
        } = self;

        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .items_center()
                    .min_w_0()
                    .child(
                        EditorButton::plain(&theme, SharedString::from(format!("{id}-done")))
                            .left_icon("icons/check.svg")
                            .label("完成")
                            .when_some(on_done, |button, handler| button.on_click(handler)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(label_color)
                            .child(label),
                    )
                    .when_some(select_all, |this, handler| {
                        this.child(
                            div()
                                .id(SharedString::from(format!("{id}-select-all")))
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(action_color)
                                .cursor_pointer()
                                .child("全选")
                                .on_click(move |event, window, cx| handler(event, window, cx)),
                        )
                    }),
            )
            .child(
                EditorButton::plain(&theme, SharedString::from(format!("{id}-delete")))
                    .danger(&theme)
                    .left_icon("icons/trash.svg")
                    .label("删除")
                    .when_some(on_delete, |button, handler| button.on_click(handler)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seven_shared_headers_pluralise_their_noun() {
        assert_eq!(selection_label("caption", 1), "已选择 1 个字幕片段");
        assert_eq!(selection_label("text", 3), "已选择 3 个文字片段");
        assert_eq!(selection_label("3D", 2), "已选择 2 个3D片段");
    }

    #[test]
    fn the_zoom_header_counts_against_the_track() {
        // `total > 1 && count === total`
        assert_eq!(zoom_selection_label(4, 4), "已全选 4 项");
        // `total > 1`
        assert_eq!(zoom_selection_label(1, 4), "已选择 1/4 项");
        // The one-segment track never says "of 1".
        assert_eq!(zoom_selection_label(1, 1), "已选择 1 项");
    }
}
