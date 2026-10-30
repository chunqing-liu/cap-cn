use crate::{
    editor_canvas::CanvasSelection, editor_panels::FieldKey, editor_window::EditorWindow, ui,
};
use cap_project::{DualDisplayAutoZoom, DualDisplayLayout, DualDisplayZoomMode, XY};
use gpui::{prelude::*, *};

#[derive(Clone, Copy)]
enum Edit {
    Layout(DualDisplayLayout),
    Canvas(u32, u32),
    Select(bool),
    Visibility(bool),
    Front(bool),
    Scale(bool, f64),
    Crop(bool, usize, f64),
    AutoZoom,
    ZoomMode(DualDisplayZoomMode),
    ZoomAmount(f64),
}

impl EditorWindow {
    pub(crate) fn clip_auto_zoom(&self, index: usize) -> Option<DualDisplayAutoZoom> {
        let dual = self.project.dual_display.as_ref()?;
        let segment = self.project.timeline.as_ref()?.segments.get(index)?;
        Some(
            segment
                .auto_zoom
                .as_ref()
                .unwrap_or(&dual.auto_zoom)
                .clone(),
        )
    }

    pub(crate) fn edit_clip_auto_zoom(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
        change: impl FnOnce(&mut DualDisplayAutoZoom),
    ) {
        let Some(default) = self
            .project
            .dual_display
            .as_ref()
            .map(|dual| dual.auto_zoom.clone())
        else {
            return;
        };
        self.edit_project("clip-auto-zoom", window, cx, move |project| {
            let Some(segment) = project
                .timeline
                .as_mut()
                .and_then(|timeline| timeline.segments.get_mut(index))
            else {
                return false;
            };
            change(segment.auto_zoom.get_or_insert(default));
            true
        });
    }

    pub(crate) fn render_dual_clip_panel(
        &self,
        index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(zoom) = self.clip_auto_zoom(index) else {
            return div().into_any_element();
        };
        let theme = self.theme;
        let mut panel = div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(format!("视频片段 {} · 独立属性", index + 1))
            .child(div().text_size(px(11.)).text_color(theme.gray_11).child(
                "以下设置只影响选中的这一段。分割会复制原设置；随后调整任意一段，其他片段不变。",
            ))
            .child(
                ui::Field::inline(&theme, "自动缩放").value(
                    ui::Toggle::plain(
                        &theme,
                        SharedString::from(format!("clip-zoom-enabled-{index}")),
                        zoom.enabled,
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.edit_clip_auto_zoom(index, window, cx, |zoom| {
                            zoom.enabled = !zoom.enabled
                        });
                    }))
                    .into_any_element(),
                ),
            )
            .child(
                ui::Field::inline(&theme, "放大倍率").value(self.render_number_field(
                    FieldKey::ClipZoomAmount(index),
                    "×",
                    96.,
                )),
            );
        for (mode, label) in [
            (DualDisplayZoomMode::FollowPointer, "鼠标跟随"),
            (DualDisplayZoomMode::FollowClicks, "点击后跟随"),
        ] {
            panel = panel.child(
                ui::Button::plain(
                    &theme,
                    SharedString::from(format!("clip-zoom-mode-{index}-{mode:?}")),
                    if zoom.mode == mode {
                        ui::ButtonVariant::Primary
                    } else {
                        ui::ButtonVariant::Gray
                    },
                    ui::ButtonSize::Sm,
                )
                .label(label)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.edit_clip_auto_zoom(index, window, cx, |zoom| zoom.mode = mode);
                })),
            );
        }
        if zoom.mode == DualDisplayZoomMode::FollowClicks {
            panel = panel.child(
                ui::Field::inline(&theme, "点击后保持").value(self.render_number_field(
                    FieldKey::ClipZoomDuration(index),
                    "秒",
                    96.,
                )),
            );
        }
        panel.child(div().text_size(px(11.)).text_color(theme.gray_11)
            .child("倍率支持 1.00–4.50×；1× 不放大。保持时长支持 0.1–60 秒。回车或离开输入框生效，随工程保存，可撤销。"))
            .into_any_element()
    }

    fn dual_control(
        &self,
        id: String,
        label: String,
        edit: Edit,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let current = self.style_control_project().dual_display.clone();
        div()
            .id(SharedString::from(id))
            .px(px(8.))
            .py(px(6.))
            .rounded(px(6.))
            .border_1()
            .border_color(self.theme.gray_6)
            .cursor_pointer()
            .child(label)
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Edit::Select(b) = edit {
                    this.canvas_selection = Some(if b {
                        CanvasSelection::Camera
                    } else {
                        CanvasSelection::Display
                    });
                    cx.notify();
                    return;
                }
                let current = current.clone();
                this.edit_controls("dual-display-settings", window, cx, move |project| {
                    let Some(dual) = &mut project.dual_display else {
                        return false;
                    };
                    match edit {
                        Edit::AutoZoom => {
                            dual.auto_zoom.enabled =
                                current.as_ref().is_none_or(|d| !d.auto_zoom.enabled);
                        }
                        Edit::ZoomMode(mode) => dual.auto_zoom.mode = mode,
                        Edit::ZoomAmount(amount) => dual.auto_zoom.amount = amount,
                        Edit::Layout(layout) => dual.apply_layout(layout),
                        Edit::Canvas(w, h) => dual.canvas_size = XY::new(w, h),
                        Edit::Select(_) => return false,
                        Edit::Front(b) => dual.a_in_front = !b,
                        Edit::Visibility(b) => {
                            let layer = if b { &mut dual.b } else { &mut dual.a };
                            layer.hidden = current
                                .as_ref()
                                .is_none_or(|d| !(if b { &d.b } else { &d.a }).hidden);
                        }
                        Edit::Scale(b, factor) => {
                            let layer = if b { &mut dual.b } else { &mut dual.a };
                            layer.size.x *= factor;
                            layer.size.y *= factor;
                            layer.sanitize();
                        }
                        Edit::Crop(b, edge, delta) => {
                            let layer = if b { &mut dual.b } else { &mut dual.a };
                            layer.crop[edge] += delta;
                            layer.sanitize();
                        }
                    }
                    true
                });
            }))
            .into_any_element()
    }

    pub(crate) fn render_dual_display_tab(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(dual) = &self.project.dual_display else {
            return div().into_any_element();
        };
        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .text_color(self.theme.gray_12)
            .child("双显示器 · 一个成片")
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(self.theme.gray_11)
                    .child("选择 A / B 后，可拖动位置和角点缩放。自动缩放只放大鼠标所在画面，保持双屏布局；无需手动关键帧。双屏模式不使用摄像头。"),
            )
            .child(self.dual_control(
                "dual-auto-zoom".into(),
                format!("自动缩放：{}", if dual.auto_zoom.enabled { "开启" } else { "关闭" }),
                Edit::AutoZoom,
                cx,
            ));
        if dual.cursor_map.is_none() {
            content = content.child("这段旧录像没有双屏鼠标数据，需用新版重新录制才能自动缩放。");
        } else {
            for (mode, name) in [
                (DualDisplayZoomMode::FollowPointer, "鼠标跟随"),
                (DualDisplayZoomMode::FollowClicks, "点击后跟随"),
            ] {
                content = content.child(self.dual_control(
                    format!("dual-zoom-mode-{mode:?}"),
                    format!(
                        "{} {name}",
                        if dual.auto_zoom.mode == mode {
                            "✓"
                        } else {
                            ""
                        }
                    ),
                    Edit::ZoomMode(mode),
                    cx,
                ));
            }
            if dual.auto_zoom.mode == DualDisplayZoomMode::FollowClicks {
                content = content.child(
                    div().flex().flex_col().gap(px(6.))
                        .child("点击后保持")
                        .child(self.render_number_field(FieldKey::DualZoomDuration, "秒", 96.))
                        .child(div().text_size(px(11.)).text_color(self.theme.gray_11)
                            .child("可输入 0.1–60 秒，支持小数；回车或离开输入框生效。再次点击重新计时。")),
                );
            }
            let mut amounts = div().flex().gap(px(6.));
            for amount in [1.0, 1.25, 1.5, 2.0] {
                amounts = amounts.child(self.dual_control(
                    format!("dual-zoom-amount-{amount}"),
                    format!(
                        "{}{amount}×",
                        if dual.auto_zoom.amount == amount {
                            "✓"
                        } else {
                            ""
                        }
                    ),
                    Edit::ZoomAmount(amount),
                    cx,
                ));
            }
            content = content.child("片段放大倍率")
                .child(self.render_number_field(FieldKey::DualZoomAmount, "×", 96.))
                .child(amounts)
                .child(
                    div()
                        .id("dual-edit-current-clip")
                        .p(px(8.))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(self.theme.gray_6)
                        .cursor_pointer()
                        .child("编辑播放位置的视频片段属性")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.select_current_dual_clip(cx);
                        })),
                )
                .child(
                    div().text_size(px(11.)).text_color(self.theme.gray_11)
                        .child("以上设置只作用于选中片段；多选可批量修改。未选片段时作用于播放头所在片段。分割后两段可分别调整。"),
                );
        }
        content = content.child(format!(
            "画布：{} × {}",
            dual.canvas_size.x, dual.canvas_size.y
        ));
        for (index, (w, h)) in [(5120, 1440), (3840, 2160), (1920, 1080), (2560, 2880)]
            .into_iter()
            .enumerate()
        {
            content = content.child(self.dual_control(
                format!("dual-canvas-{index}"),
                format!("{w} × {h}"),
                Edit::Canvas(w, h),
                cx,
            ));
        }
        let mut presets = div().flex().gap(px(6.));
        for (index, (name, layout)) in [
            ("左右并排", DualDisplayLayout::SideBySide),
            ("上下排列", DualDisplayLayout::Stacked),
            ("画中画", DualDisplayLayout::PictureInPicture),
        ]
        .into_iter()
        .enumerate()
        {
            presets = presets.child(self.dual_control(
                format!("dual-layout-{index}"),
                name.into(),
                Edit::Layout(layout),
                cx,
            ));
        }
        content = content.child(presets);
        for b in [false, true] {
            let letter = if b { "B" } else { "A" };
            let layer = if b { &dual.b } else { &dual.a };
            let name = if b { &dual.source_b } else { &dual.source_a };
            let mut card = div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .pt(px(12.))
                .child(format!("显示器 {letter} · {name}"))
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(self.dual_control(
                            format!("dual-select-{letter}"),
                            "选择画面".into(),
                            Edit::Select(b),
                            cx,
                        ))
                        .child(self.dual_control(
                            format!("dual-visible-{letter}"),
                            if layer.hidden { "显示" } else { "隐藏" }.into(),
                            Edit::Visibility(b),
                            cx,
                        ))
                        .child(self.dual_control(
                            format!("dual-front-{letter}"),
                            "置于顶层".into(),
                            Edit::Front(b),
                            cx,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(self.dual_control(
                            format!("dual-smaller-{letter}"),
                            "缩小".into(),
                            Edit::Scale(b, 0.9),
                            cx,
                        ))
                        .child(self.dual_control(
                            format!("dual-larger-{letter}"),
                            "放大".into(),
                            Edit::Scale(b, 1.1),
                            cx,
                        )),
                );
            for (edge, label) in ["左", "上", "右", "下"].into_iter().enumerate() {
                card = card.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(format!("{label}裁剪 {:.0}%", layer.crop[edge] * 100.0))
                        .child(self.dual_control(
                            format!("dual-crop-minus-{letter}-{edge}"),
                            "−".into(),
                            Edit::Crop(b, edge, -0.01),
                            cx,
                        ))
                        .child(self.dual_control(
                            format!("dual-crop-plus-{letter}-{edge}"),
                            "＋".into(),
                            Edit::Crop(b, edge, 0.01),
                            cx,
                        )),
                );
            }
            content = content.child(card);
        }
        content.into_any_element()
    }
}
