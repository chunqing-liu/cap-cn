use crate::XY;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;

pub const DUAL_DISPLAY_SOURCES_FILE: &str = "dual-display-sources.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplayLayerConfiguration {
    pub center: XY<f64>,
    pub size: XY<f64>,
    pub crop: [f64; 4],
    pub hidden: bool,
}

impl Default for DisplayLayerConfiguration {
    fn default() -> Self {
        Self {
            center: XY::new(0.5, 0.5),
            size: XY::new(1.0, 1.0),
            crop: [0.0; 4],
            hidden: false,
        }
    }
}

impl DisplayLayerConfiguration {
    pub fn sanitize(&mut self) {
        fn finite(value: f64, fallback: f64, min: f64, max: f64) -> f64 {
            if value.is_finite() {
                value.clamp(min, max)
            } else {
                fallback
            }
        }
        self.center.x = finite(self.center.x, 0.5, 0.0, 1.0);
        self.center.y = finite(self.center.y, 0.5, 0.0, 1.0);
        self.size.x = finite(self.size.x, 1.0, 0.02, 1.0);
        self.size.y = finite(self.size.y, 1.0, 0.02, 1.0);
        for value in &mut self.crop {
            *value = finite(*value, 0.0, 0.0, 0.45);
        }
    }

    pub fn fitted_bounds(&self, output: XY<u32>, source: XY<u32>) -> [f32; 4] {
        let mut layer = self.clone();
        layer.sanitize();
        let width = f64::from(output.x.max(1));
        let height = f64::from(output.y.max(1));
        let available_w = width * layer.size.x;
        let available_h = height * layer.size.y;
        let source_w = f64::from(source.x.max(1)) * (1.0 - layer.crop[0] - layer.crop[2]);
        let source_h = f64::from(source.y.max(1)) * (1.0 - layer.crop[1] - layer.crop[3]);
        let scale = (available_w / source_w).min(available_h / source_h);
        let w = source_w * scale;
        let h = source_h * scale;
        let left = (width * layer.center.x - w / 2.0).clamp(0.0, (width - w).max(0.0));
        let top = (height * layer.center.y - h / 2.0).clamp(0.0, (height - h).max(0.0));
        [left as f32, top as f32, (left + w) as f32, (top + h) as f32]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DualDisplayLayout {
    SideBySide,
    Stacked,
    PictureInPicture,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DualDisplayCursorMap {
    pub b_origin: XY<f64>,
    pub b_size: XY<f64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DualDisplayZoomMode {
    #[default]
    FollowPointer,
    FollowClicks,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct DualDisplayAutoZoom {
    pub enabled: bool,
    pub amount: f64,
    pub mode: DualDisplayZoomMode,
    pub hold_seconds: f64,
}

impl Default for DualDisplayAutoZoom {
    fn default() -> Self {
        Self {
            enabled: true,
            amount: 1.75,
            mode: DualDisplayZoomMode::FollowPointer,
            hold_seconds: 2.5,
        }
    }
}

impl DualDisplayAutoZoom {
    pub fn click_hold_seconds(&self) -> f64 {
        if self.hold_seconds.is_finite() {
            self.hold_seconds.clamp(0.1, 60.0)
        } else {
            2.5
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct DualDisplayConfiguration {
    pub canvas_size: XY<u32>,
    pub source_a: String,
    pub source_b: String,
    pub a: DisplayLayerConfiguration,
    pub b: DisplayLayerConfiguration,
    pub a_in_front: bool,
    pub cursor_map: Option<DualDisplayCursorMap>,
    pub cursor_baked_in: bool,
    pub auto_zoom: DualDisplayAutoZoom,
}

impl Default for DualDisplayConfiguration {
    fn default() -> Self {
        let mut config = Self {
            canvas_size: XY::new(5120, 1440),
            source_a: "显示器 A".into(),
            source_b: "显示器 B".into(),
            a: DisplayLayerConfiguration::default(),
            b: DisplayLayerConfiguration::default(),
            a_in_front: false,
            cursor_map: None,
            cursor_baked_in: true,
            auto_zoom: DualDisplayAutoZoom::default(),
        };
        config.apply_layout(DualDisplayLayout::SideBySide);
        config
    }
}

impl DualDisplayConfiguration {
    pub fn for_clip(&self, segment: Option<&crate::TimelineSegment>) -> Self {
        let mut config = self.clone();
        if let Some(zoom) = segment.and_then(|segment| segment.auto_zoom.as_ref()) {
            config.auto_zoom = zoom.clone();
        }
        config
    }

    pub fn cursor_position(&self, b: bool, x: f64, y: f64) -> Option<XY<f64>> {
        let map = self.cursor_map.as_ref()?;
        let position = if b {
            XY::new(
                (x - map.b_origin.x) / map.b_size.x,
                (y - map.b_origin.y) / map.b_size.y,
            )
        } else {
            XY::new(x, y)
        };
        (position.x.is_finite()
            && position.y.is_finite()
            && (0.0..1.0).contains(&position.x)
            && (0.0..1.0).contains(&position.y))
        .then_some(position)
    }

    pub fn zoom_crop(&self, b: bool, events: &crate::CursorEvents, time: f64) -> [f64; 4] {
        self.zoom_crop_with_amount(b, events, time, self.zoom_amount_at(&[], time))
    }

    pub fn cursor_source_position(&self, x: f64, y: f64) -> Option<XY<f64>> {
        self.cursor_source_position_for_target(crate::ZoomTargetDisplay::FollowCursor, x, y)
    }

    pub fn cursor_source_position_for_target(
        &self,
        target: crate::ZoomTargetDisplay,
        x: f64,
        y: f64,
    ) -> Option<XY<f64>> {
        let b = match target {
            crate::ZoomTargetDisplay::A => false,
            crate::ZoomTargetDisplay::B => true,
            crate::ZoomTargetDisplay::FollowCursor => self.cursor_position(false, x, y).is_none(),
        };
        let position = self.cursor_position(b, x, y)?;
        let mut layer = if b { self.b.clone() } else { self.a.clone() };
        layer.sanitize();
        if layer.hidden {
            return None;
        }
        let u =
            ((position.x - layer.crop[0]) / (1.0 - layer.crop[0] - layer.crop[2])).clamp(0.0, 1.0);
        let v =
            ((position.y - layer.crop[1]) / (1.0 - layer.crop[1] - layer.crop[3])).clamp(0.0, 1.0);
        Some(XY::new(u, v))
    }

    fn current_cursor_position(
        &self,
        b: bool,
        events: &crate::CursorEvents,
        time: f64,
    ) -> Option<XY<f64>> {
        if !time.is_finite() {
            return None;
        }
        let index = events
            .moves
            .partition_point(|event| event.time_ms <= time * 1000.0);
        let event = index.checked_sub(1).and_then(|i| events.moves.get(i))?;
        self.cursor_position(b, event.x, event.y)
    }

    pub fn zoom_crop_with_track(
        &self,
        b: bool,
        events: &crate::CursorEvents,
        time: f64,
        track_crop: [f64; 4],
        activity: f64,
    ) -> [f64; 4] {
        self.zoom_crop_with_targeted_track(
            b,
            events,
            time,
            track_crop,
            activity,
            crate::ZoomTargetDisplay::FollowCursor,
        )
    }

    pub fn zoom_crop_with_targeted_track(
        &self,
        b: bool,
        events: &crate::CursorEvents,
        time: f64,
        track_crop: [f64; 4],
        activity: f64,
        target: crate::ZoomTargetDisplay,
    ) -> [f64; 4] {
        let mut layer = if b { self.b.clone() } else { self.a.clone() };
        layer.sanitize();
        let base = [
            layer.crop[0],
            layer.crop[1],
            1.0 - layer.crop[2],
            1.0 - layer.crop[3],
        ];
        if layer.hidden {
            return base;
        }
        let auto = self.zoom_crop(b, events, time);
        if !track_crop.iter().all(|value| value.is_finite()) || !activity.is_finite() {
            return auto;
        }
        let track_engaged = activity > 0.0001 || track_crop[2] - track_crop[0] < 0.9999;
        if !track_engaged {
            return auto;
        }
        let selected = match target {
            crate::ZoomTargetDisplay::FollowCursor => {
                self.current_cursor_position(b, events, time).is_some()
            }
            crate::ZoomTargetDisplay::A => !b,
            crate::ZoomTargetDisplay::B => b,
        };
        if !selected {
            return base;
        }
        let track_width = (track_crop[2] - track_crop[0]).clamp(1.0 / 16.0, 1.0);
        let track_amount = 1.0 / track_width;
        let auto_amount = (base[2] - base[0]) / (auto[2] - auto[0]);
        let amount = track_amount + (auto_amount - 1.0) * (1.0 - activity.clamp(0.0, 1.0));
        if amount <= 1.0 {
            return base;
        }
        if activity <= 0.0 && track_amount <= 1.0 {
            return auto;
        }
        let track_weight = if amount > 1.0 {
            ((track_amount - 1.0) / (amount - 1.0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let axis = |start: usize, end: usize| {
            let size = base[end] - base[start];
            let track_size = (track_crop[end] - track_crop[start]).clamp(1.0 / 16.0, 1.0);
            let track_center = if track_size < 1.0 {
                (track_crop[start] / (1.0 - track_size)).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let auto_travel = size - auto[end] + auto[start];
            let auto_center = if auto_travel > f64::EPSILON {
                ((auto[start] - base[start]) / auto_travel).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let center = auto_center + (track_center - auto_center) * track_weight;
            let visible = size / amount.max(1.0);
            let offset = base[start] + center * (size - visible);
            (offset, offset + visible)
        };
        let (left, right) = axis(0, 2);
        let (top, bottom) = axis(1, 3);
        [left, top, right, bottom]
    }

    pub fn zoom_amount_at(&self, segments: &[crate::ZoomSegment], time: f64) -> f64 {
        let base = if self.auto_zoom.amount.is_finite() {
            self.auto_zoom.amount.clamp(1.0, 4.5)
        } else {
            1.75
        };
        let Some(segment) = segments
            .iter()
            .find(|segment| time >= segment.start && time < segment.end)
        else {
            return base;
        };
        let fade = 0.2_f64.min((segment.end - segment.start) / 2.0);
        let blend =
            ((time - segment.start).min(segment.end - time) / fade.max(0.001)).clamp(0.0, 1.0);
        let blend = blend * blend * (3.0 - 2.0 * blend);
        let amount = if segment.amount.is_finite() {
            segment.amount.clamp(1.0, 4.5)
        } else {
            base
        };
        base + (amount - base) * blend
    }

    pub fn zoom_crop_with_amount(
        &self,
        b: bool,
        events: &crate::CursorEvents,
        time: f64,
        amount: f64,
    ) -> [f64; 4] {
        let mut layer = if b { self.b.clone() } else { self.a.clone() };
        layer.sanitize();
        let base = [
            layer.crop[0],
            layer.crop[1],
            1.0 - layer.crop[2],
            1.0 - layer.crop[3],
        ];
        if !self.auto_zoom.enabled
            || layer.hidden
            || self.current_cursor_position(b, events, time).is_none()
        {
            return base;
        }
        let position_at = |time: f64| {
            let index = events
                .moves
                .partition_point(|event| event.time_ms <= time * 1000.0);
            let event = index.checked_sub(1).and_then(|i| events.moves.get(i))?;
            self.cursor_position(b, event.x, event.y)
                .filter(|p| p.x >= base[0] && p.y >= base[1] && p.x < base[2] && p.y < base[3])
        };
        let mut active = 0.0;
        let start = time - 0.6;
        let hold_seconds = self.auto_zoom.click_hold_seconds();
        let total = 1.0 - (-0.6_f64 / 0.16).exp();
        let mut boundaries = vec![start, time];
        let first = events
            .moves
            .partition_point(|event| event.time_ms < start * 1000.0);
        let last = events
            .moves
            .partition_point(|event| event.time_ms <= time * 1000.0);
        boundaries.extend(
            events.moves[first..last]
                .iter()
                .map(|event| event.time_ms / 1000.0),
        );
        if self.auto_zoom.mode == DualDisplayZoomMode::FollowClicks {
            let first = events
                .clicks
                .partition_point(|event| event.time_ms < (start - hold_seconds) * 1000.0);
            let last = events
                .clicks
                .partition_point(|event| event.time_ms <= time * 1000.0);
            for click in events.clicks[first..last].iter().filter(|event| event.down) {
                for boundary in [
                    click.time_ms / 1000.0,
                    click.time_ms / 1000.0 + hold_seconds,
                ] {
                    if boundary > start && boundary < time {
                        boundaries.push(boundary);
                    }
                }
            }
        }
        boundaries.sort_by(f64::total_cmp);
        boundaries.dedup();
        let mut focus = XY::new(0.0, 0.0);
        for interval in boundaries.windows(2) {
            let sample_time = (interval[0] + interval[1]) / 2.0;
            let weight = ((interval[1] - time) / 0.16).exp() - ((interval[0] - time) / 0.16).exp();
            let Some(position) = position_at(sample_time) else {
                continue;
            };
            if self.auto_zoom.mode == DualDisplayZoomMode::FollowClicks {
                let end = events
                    .clicks
                    .partition_point(|event| event.time_ms <= sample_time * 1000.0);
                let click = events.clicks[..end].iter().rev().find(|event| event.down);
                if !click.is_some_and(|click| {
                    sample_time * 1000.0 - click.time_ms < hold_seconds * 1000.0
                        && position_at(click.time_ms / 1000.0).is_some()
                }) {
                    continue;
                }
            }
            active += weight;
            focus.x += position.x * weight;
            focus.y += position.y * weight;
        }
        if active <= f64::EPSILON {
            return base;
        }
        let amount = if amount.is_finite() {
            amount.clamp(1.0, 4.5)
        } else {
            1.75
        };
        let zoom = 1.0 + (amount - 1.0) * (active / total).clamp(0.0, 1.0);
        let width = (base[2] - base[0]) / zoom;
        let height = (base[3] - base[1]) / zoom;
        let left = (focus.x / active - width / 2.0).clamp(base[0], (base[2] - width).max(base[0]));
        let top = (focus.y / active - height / 2.0).clamp(base[1], (base[3] - height).max(base[1]));
        [
            left,
            top,
            (left + width).min(base[2]),
            (top + height).min(base[3]),
        ]
    }

    pub fn apply_layout(&mut self, layout: DualDisplayLayout) {
        self.a = DisplayLayerConfiguration::default();
        self.b = DisplayLayerConfiguration::default();
        self.a_in_front = false;
        match layout {
            DualDisplayLayout::SideBySide => {
                self.a.center.x = 0.25;
                self.b.center.x = 0.75;
                self.a.size = XY::new(0.5, 1.0);
                self.b.size = XY::new(0.5, 1.0);
            }
            DualDisplayLayout::Stacked => {
                self.a.center.y = 0.25;
                self.b.center.y = 0.75;
                self.a.size = XY::new(1.0, 0.5);
                self.b.size = XY::new(1.0, 0.5);
            }
            DualDisplayLayout::PictureInPicture => {
                self.b.center = XY::new(0.79, 0.79);
                self.b.size = XY::new(0.36, 0.36);
            }
        }
    }

    pub fn load_sources(project: impl AsRef<Path>) -> std::io::Result<Option<Self>> {
        match std::fs::read(project.as_ref().join(DUAL_DISPLAY_SOURCES_FILE)) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn write_sources(&self, project: impl AsRef<Path>) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        std::fs::create_dir_all(project.as_ref())?;
        std::fs::write(project.as_ref().join(DUAL_DISPLAY_SOURCES_FILE), bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectConfiguration;

    #[test]
    fn zoom_track_maps_each_cursor_to_its_own_display() {
        let mut config = tracked_project();
        assert_eq!(
            config.cursor_source_position(0.5, 0.5),
            Some(XY::new(0.5, 0.5))
        );
        assert_eq!(
            config.cursor_source_position(-0.5, 0.5),
            Some(XY::new(0.5, 0.5))
        );
        config.apply_layout(DualDisplayLayout::Stacked);
        config.canvas_size = XY::new(2560, 2880);
        assert_eq!(
            config.cursor_source_position(-0.5, 0.5),
            Some(XY::new(0.5, 0.5))
        );
        config.b.hidden = true;
        assert_eq!(config.cursor_source_position(-0.5, 0.5), None);
    }

    #[test]
    fn manual_track_only_crops_the_current_display_even_with_auto_zoom_off() {
        let mut config = tracked_project();
        let events = moves(&[(0.0, 0.5, 0.5), (2.0, -0.5, 0.5)]);
        let base = [0.0, 0.0, 1.0, 1.0];
        let track = [0.5, 0.0, 1.0, 0.5];
        for enabled in [false, true] {
            config.auto_zoom.enabled = enabled;
            for (time, current) in [(1.0, false), (2.0, true), (2.01, true), (3.0, true)] {
                assert_eq!(
                    config.zoom_crop_with_track(current, &events, time, track, 1.0),
                    track
                );
                assert_eq!(
                    config.zoom_crop_with_track(!current, &events, time, track, 1.0),
                    base
                );
            }
        }
    }

    #[test]
    fn track_zoom_keeps_static_crop_and_falls_back_to_clip_zoom() {
        let mut config = tracked_project();
        config.b.crop = [0.1, 0.2, 0.2, 0.1];
        let events = moves(&[(0.0, -0.5, 0.5)]);
        let track = [0.0, 0.5, 0.5, 1.0];
        let crop = config.zoom_crop_with_track(true, &events, 1.0, track, 1.0);
        for (actual, expected) in crop.into_iter().zip([0.1, 0.55, 0.45, 0.9]) {
            assert!((actual - expected).abs() < 1e-9);
        }
        assert_eq!(
            config.zoom_crop_with_track(false, &events, 1.0, track, 1.0),
            [0.0, 0.0, 1.0, 1.0]
        );
        let auto = config.zoom_crop(true, &events, 1.0);
        let fallback = config.zoom_crop_with_track(true, &events, 1.0, [0.0, 0.0, 1.0, 1.0], 0.0);
        for (actual, expected) in fallback.into_iter().zip(auto) {
            assert!((actual - expected).abs() < 1e-9);
        }
        assert_eq!(
            config.zoom_crop_with_track(true, &events, 1.0, [0.0, 0.0, 1.0, 1.0], 1.0),
            [0.1, 0.2, 0.8, 0.9]
        );
    }

    #[test]
    fn source_selection_does_not_leave_zoom_on_the_outgoing_display() {
        let config = tracked_project();
        let events = moves(&[(0.0, 0.5, 0.5), (2.0, -0.5, 0.5), (3.0, 4.0, 4.0)]);
        let base = [0.0, 0.0, 1.0, 1.0];
        for time in [2.0, 2.001, 2.1, 2.5, 3.0] {
            assert_eq!(config.zoom_crop(false, &events, time), base);
        }
        assert_eq!(config.zoom_crop(true, &events, 3.0), base);
        for b in [false, true] {
            assert_eq!(
                config.zoom_crop_with_track(b, &events, 3.0, [0.0, 0.0, 0.5, 0.5], 1.0),
                base
            );
        }
    }

    #[test]
    fn custom_zoom_ranges_preserve_the_global_amount_and_allow_one_x() {
        let config = tracked_project();
        let ranges: Vec<crate::ZoomSegment> = serde_json::from_value(serde_json::json!([
            {"start": 10.0, "end": 20.0, "amount": 1.1, "mode": "auto"},
            {"start": 30.0, "end": 40.0, "amount": 1.0, "mode": "auto"}
        ]))
        .unwrap();
        assert_eq!(config.zoom_amount_at(&ranges, 5.0), 1.75);
        assert!((config.zoom_amount_at(&ranges, 15.0) - 1.1).abs() < 0.0001);
        assert_eq!(config.zoom_amount_at(&ranges, 35.0), 1.0);
        assert_eq!(config.zoom_amount_at(&ranges, 40.0), 1.75);
        assert_eq!(config.zoom_amount_at(&ranges, 10.0), 1.75);
        assert!(config.zoom_amount_at(&ranges, 10.1) > 1.1);
        let events = moves(&[(0.0, 0.5, 0.5)]);
        let crop = config.zoom_crop_with_amount(false, &events, 15.0, 1.1);
        assert!((crop[2] - crop[0] - 1.0 / 1.1).abs() < 0.0001);
        assert_eq!(
            config.zoom_crop_with_amount(false, &events, 35.0, 1.0),
            [0.0, 0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn legacy_dual_recordings_keep_their_baked_cursor_marker() {
        let legacy: DualDisplayConfiguration = serde_json::from_str("{}").unwrap();
        assert!(legacy.cursor_baked_in);
        let current = DualDisplayConfiguration {
            cursor_baked_in: false,
            ..legacy
        };
        let restored: DualDisplayConfiguration =
            serde_json::from_str(&serde_json::to_string(&current).unwrap()).unwrap();
        assert!(!restored.cursor_baked_in);
    }

    fn tracked_project() -> DualDisplayConfiguration {
        DualDisplayConfiguration {
            cursor_map: Some(DualDisplayCursorMap {
                b_origin: XY::new(-1.0, 0.0),
                b_size: XY::new(1.0, 1.0),
            }),
            ..Default::default()
        }
    }

    fn moves(positions: &[(f64, f64, f64)]) -> crate::CursorEvents {
        crate::CursorEvents {
            moves: positions
                .iter()
                .map(|&(time, x, y)| crate::CursorMoveEvent {
                    time_ms: time * 1000.0,
                    x,
                    y,
                    cursor_id: "0".into(),
                    active_modifiers: vec![],
                })
                .collect(),
            clicks: vec![],
        }
    }

    #[test]
    fn pointer_zoom_switches_sources_without_changing_layer_bounds() {
        let config = tracked_project();
        let events = moves(&[(0.0, 0.5, 0.5), (2.0, -0.25, 0.5)]);
        let a = config.zoom_crop(false, &events, 1.0);
        assert!((a[2] - a[0] - 1.0 / 1.75).abs() < 0.0001);
        assert_eq!(config.zoom_crop(true, &events, 1.0), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(config.zoom_crop(false, &events, 3.0), [0.0, 0.0, 1.0, 1.0]);
        assert!(config.zoom_crop(true, &events, 3.0)[0] > 0.4);
        let before = config
            .a
            .fitted_bounds(config.canvas_size, XY::new(2560, 1440));
        assert_eq!(before, [0.0, 0.0, 2560.0, 1440.0]);
        let transition = config.zoom_crop(true, &events, 2.1);
        assert!(transition[2] - transition[0] > 1.0 / 1.75);
        assert!(transition[2] - transition[0] < 1.0);
    }

    #[test]
    fn incoming_zoom_is_continuous_without_zooming_the_outgoing_source() {
        let config = tracked_project();
        let events = moves(&[(0.0, 0.5, 0.5), (2.0, -0.5, 0.5)]);
        assert_eq!(config.zoom_crop(false, &events, 0.0), [0.0, 0.0, 1.0, 1.0]);
        let before = config.zoom_crop(true, &events, 2.0 - 0.00001);
        let after = config.zoom_crop(true, &events, 2.0 + 0.00001);
        assert!((before[2] - before[0] - after[2] + after[0]).abs() < 0.001);
        assert_eq!(config.zoom_crop(false, &events, 2.0), [0.0, 0.0, 1.0, 1.0]);
        let entry = config.zoom_crop(false, &events, 0.001);
        assert!(entry[2] - entry[0] > 0.99);
    }

    #[test]
    fn physical_cursor_map_handles_vertical_offset_and_different_screen_sizes() {
        let mut config = tracked_project();
        config.cursor_map = Some(DualDisplayCursorMap {
            b_origin: XY::new(1.0, -0.5),
            b_size: XY::new(1.5, 1.5),
        });
        assert_eq!(
            config.cursor_position(true, 1.75, 0.25),
            Some(XY::new(0.5, 0.5))
        );
        assert_eq!(config.cursor_position(false, 1.75, 0.25), None);
        assert_eq!(config.cursor_position(true, 0.5, 0.5), None);
    }

    #[test]
    fn zoom_is_optional_and_old_recordings_do_not_invent_mouse_data() {
        let events = moves(&[(0.0, 0.5, 0.5)]);
        let base = [0.0, 0.0, 1.0, 1.0];
        let old: DualDisplayConfiguration = serde_json::from_str("{}").unwrap();
        assert_eq!(old.zoom_crop(false, &events, 1.0), base);
        let mut config = tracked_project();
        config.auto_zoom.enabled = false;
        assert_eq!(config.zoom_crop(false, &events, 1.0), base);
        config.auto_zoom.enabled = true;
        assert_eq!(
            config.zoom_crop(false, &crate::CursorEvents::default(), 1.0),
            base
        );
        assert_eq!(config.zoom_crop(false, &events, -1.0), base);
    }

    #[test]
    fn click_follow_only_zooms_the_clicked_display_then_restores_it() {
        let mut config = tracked_project();
        config.auto_zoom.mode = DualDisplayZoomMode::FollowClicks;
        let mut events = moves(&[(0.0, -0.5, 0.5), (2.0, 0.5, 0.5)]);
        events.clicks.push(crate::CursorClickEvent {
            time_ms: 500.0,
            down: true,
            active_modifiers: vec![],
            cursor_num: 1,
            cursor_id: "0".into(),
        });
        assert_eq!(config.zoom_crop(true, &events, 0.4), [0.0, 0.0, 1.0, 1.0]);
        assert!(config.zoom_crop(true, &events, 1.5)[0] > 0.1);
        assert_eq!(config.zoom_crop(false, &events, 2.8), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(config.zoom_crop(true, &events, 4.0), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn custom_click_hold_duration_is_saved_and_repeated_clicks_restart_it() {
        let mut config = tracked_project();
        config.auto_zoom.mode = DualDisplayZoomMode::FollowClicks;
        config.auto_zoom.hold_seconds = 5.5;
        let mut events = moves(&[(0.0, 0.5, 0.5)]);
        for time_ms in [500.0, 5000.0] {
            events.clicks.push(crate::CursorClickEvent {
                time_ms,
                down: true,
                active_modifiers: vec![],
                cursor_num: 1,
                cursor_id: "0".into(),
            });
        }
        assert!(config.zoom_crop(false, &events, 4.0)[0] > 0.1);
        assert!(config.zoom_crop(false, &events, 10.0)[0] > 0.1);
        assert_eq!(config.zoom_crop(false, &events, 11.2), [0.0, 0.0, 1.0, 1.0]);
        let restored: DualDisplayConfiguration =
            serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert_eq!(restored.auto_zoom.click_hold_seconds(), 5.5);
        let old: DualDisplayAutoZoom = serde_json::from_str(r#"{"mode":"followClicks"}"#).unwrap();
        assert_eq!(old.click_hold_seconds(), 2.5);
        config.auto_zoom.hold_seconds = f64::NAN;
        assert_eq!(config.auto_zoom.click_hold_seconds(), 2.5);
        config.auto_zoom.hold_seconds = -1.0;
        assert_eq!(config.auto_zoom.click_hold_seconds(), 0.1);
    }

    #[test]
    fn zoom_stays_inside_static_crop_and_survives_random_access_and_serialization() {
        let mut config = tracked_project();
        config.a.crop = [0.1, 0.15, 0.2, 0.1];
        config.auto_zoom.amount = 2.5;
        let events = moves(&[(0.0, 0.11, 0.16), (1.0, 0.78, 0.88)]);
        let bounds = config.zoom_crop(false, &events, 1.5);
        assert!(bounds[0] >= 0.1 && bounds[1] >= 0.15 && bounds[2] <= 0.8 && bounds[3] <= 0.9);
        let _earlier = config.zoom_crop(false, &events, 0.5);
        assert_eq!(bounds, config.zoom_crop(false, &events, 1.5));
        let restored: DualDisplayConfiguration =
            serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert_eq!(bounds, restored.zoom_crop(false, &events, 1.5));
    }

    #[test]
    fn two_qhd_displays_fill_5120_canvas_exactly() {
        let config = DualDisplayConfiguration::default();
        let source = XY::new(2560, 1440);
        assert_eq!(
            config.a.fitted_bounds(config.canvas_size, source),
            [0.0, 0.0, 2560.0, 1440.0]
        );
        assert_eq!(
            config.b.fitted_bounds(config.canvas_size, source),
            [2560.0, 0.0, 5120.0, 1440.0]
        );
    }

    #[test]
    fn mixed_resolution_screens_fit_without_stretching() {
        let config = DualDisplayConfiguration::default();
        let output = XY::new(1920, 1080);
        let a = config.a.fitted_bounds(output, XY::new(3840, 2160));
        let b = config.b.fitted_bounds(output, XY::new(1080, 1920));
        assert!(a[2] <= b[0]);
        assert!(((a[2] - a[0]) / (a[3] - a[1]) - 16.0 / 9.0).abs() < 0.001);
        assert!(((b[2] - b[0]) / (b[3] - b[1]) - 9.0 / 16.0).abs() < 0.001);
    }

    #[test]
    fn transforms_remain_finite_and_on_canvas() {
        let layer = DisplayLayerConfiguration {
            center: XY::new(f64::NAN, 5.0),
            size: XY::new(8.0, -2.0),
            crop: [0.9; 4],
            hidden: false,
        };
        let bounds = layer.fitted_bounds(XY::new(1920, 1080), XY::new(1920, 1080));
        assert!(bounds.iter().all(|value| value.is_finite()));
        assert!(bounds[0] >= 0.0 && bounds[1] >= 0.0);
        assert!(bounds[2] <= 1920.0 && bounds[3] <= 1080.0);
    }

    #[test]
    fn independent_transforms_survive_serialization() {
        let mut config = DualDisplayConfiguration::default();
        config.a.hidden = true;
        config.b.crop[0] = 0.1;
        config.b.center = XY::new(0.7, 0.3);
        config.a_in_front = true;
        let restored: DualDisplayConfiguration =
            serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert!(restored.a.hidden && restored.a_in_front);
        assert_eq!(restored.b.crop[0], 0.1);
        assert_eq!(restored.b.center.x, 0.7);
    }

    #[test]
    fn old_projects_stay_single_source_and_marked_projects_recover_roles() {
        let project = tempfile::tempdir().unwrap();
        ProjectConfiguration::default()
            .write(project.path())
            .unwrap();
        assert!(
            ProjectConfiguration::load(project.path())
                .unwrap()
                .dual_display
                .is_none()
        );
        DualDisplayConfiguration::default()
            .write_sources(project.path())
            .unwrap();
        let mut restored = ProjectConfiguration::load(project.path()).unwrap();
        restored.dual_display.as_mut().unwrap().b.center.x = 0.4;
        restored.write(project.path()).unwrap();
        assert_eq!(
            ProjectConfiguration::load(project.path())
                .unwrap()
                .dual_display
                .unwrap()
                .b
                .center
                .x,
            0.4
        );
    }

    #[test]
    fn invalid_source_marker_is_not_interpreted_as_a_webcam_project() {
        let project = tempfile::tempdir().unwrap();
        ProjectConfiguration::default()
            .write(project.path())
            .unwrap();
        std::fs::write(project.path().join(DUAL_DISPLAY_SOURCES_FILE), b"{").unwrap();
        assert_eq!(
            ProjectConfiguration::load(project.path())
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn source_marker_can_create_a_new_recording_directory() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("new-recording.cap");
        DualDisplayConfiguration::default()
            .write_sources(&project)
            .unwrap();
        assert_eq!(
            DualDisplayConfiguration::load_sources(&project)
                .unwrap()
                .unwrap()
                .canvas_size,
            XY::new(5120, 1440)
        );
    }

    #[test]
    fn fixed_track_target_overrides_the_cursor_and_suppresses_other_auto_zoom() {
        for enabled in [false, true] {
            let mut config = tracked_project();
            config.auto_zoom.enabled = enabled;
            for (target, selected_b, events) in [
                (
                    crate::ZoomTargetDisplay::A,
                    false,
                    moves(&[(0.0, -0.2, 0.75)]),
                ),
                (
                    crate::ZoomTargetDisplay::B,
                    true,
                    moves(&[(0.0, 0.15, 0.2)]),
                ),
            ] {
                for activity in [0.2, 1.0] {
                    let track = [0.0, 0.0, 0.5, 0.5];
                    assert_eq!(
                        config.zoom_crop_with_targeted_track(
                            selected_b, &events, 1.0, track, activity, target
                        ),
                        track
                    );
                    assert_eq!(
                        config.zoom_crop_with_targeted_track(
                            !selected_b,
                            &events,
                            1.0,
                            track,
                            activity,
                            target
                        ),
                        [0.0, 0.0, 1.0, 1.0]
                    );
                }
                assert_eq!(
                    config.zoom_crop_with_targeted_track(
                        !selected_b,
                        &events,
                        1.0,
                        [0.0, 0.0, 1.0, 1.0],
                        0.0,
                        target
                    ),
                    config.zoom_crop(!selected_b, &events, 1.0)
                );
                assert_eq!(
                    config.zoom_crop_with_targeted_track(
                        !selected_b,
                        &events,
                        1.0,
                        [0.0, 0.0, 1.0, 1.0],
                        1.0,
                        target
                    ),
                    [0.0, 0.0, 1.0, 1.0]
                );
            }
        }
    }

    #[test]
    fn fixed_manual_target_works_without_cursor_capture_and_preserves_static_crop() {
        let mut config = tracked_project();
        config.b.crop = [0.1, 0.1, 0.2, 0.2];
        let events = crate::CursorEvents::default();
        let crop = config.zoom_crop_with_targeted_track(
            true,
            &events,
            1.0,
            [0.5, 0.5, 1.0, 1.0],
            1.0,
            crate::ZoomTargetDisplay::B,
        );
        for (actual, expected) in crop.into_iter().zip([0.45, 0.45, 0.8, 0.8]) {
            assert!((actual - expected).abs() < 0.00001);
        }
        assert_eq!(
            config.zoom_crop_with_targeted_track(
                false,
                &events,
                1.0,
                [0.5, 0.5, 1.0, 1.0],
                1.0,
                crate::ZoomTargetDisplay::B
            ),
            [0.0, 0.0, 1.0, 1.0]
        );
    }

    #[test]
    fn zoom_target_defaults_to_cursor_and_survives_independent_clone_and_save() {
        let legacy: crate::ZoomSegment =
            serde_json::from_str(r#"{"start":1,"end":5,"amount":2,"mode":"auto"}"#).unwrap();
        assert_eq!(
            legacy.target_display,
            crate::ZoomTargetDisplay::FollowCursor
        );
        let mut left = legacy.clone();
        let mut right = legacy.clone();
        left.end = 3.0;
        left.target_display = crate::ZoomTargetDisplay::A;
        right.start = 3.0;
        right.target_display = crate::ZoomTargetDisplay::B;
        for segment in [left, right] {
            let restored: crate::ZoomSegment =
                serde_json::from_slice(&serde_json::to_vec(&segment).unwrap()).unwrap();
            assert_eq!(segment, restored);
        }
        assert_eq!(
            legacy.target_display,
            crate::ZoomTargetDisplay::FollowCursor
        );
    }

    #[test]
    fn video_clips_keep_independent_auto_zoom_after_copy_and_round_trip() {
        let global = tracked_project();
        let original: crate::TimelineSegment = serde_json::from_str(
            r#"{"start":0,"end":6,"timescale":1,"autoZoom":{"enabled":true,"amount":1.1,"mode":"followPointer","holdSeconds":4.2}}"#
        ).unwrap();
        let mut left = original.clone();
        left.end = 3.0;
        let mut right = original.clone();
        right.start = 3.0;
        right.auto_zoom.as_mut().unwrap().enabled = false;
        right.auto_zoom.as_mut().unwrap().mode = DualDisplayZoomMode::FollowClicks;
        right.auto_zoom.as_mut().unwrap().hold_seconds = 8.5;
        left.auto_zoom.as_mut().unwrap().amount = 1.25;
        assert_eq!(original.auto_zoom.as_ref().unwrap().amount, 1.1);
        assert_eq!(right.auto_zoom.as_ref().unwrap().amount, 1.1);
        assert!(left.auto_zoom.as_ref().unwrap().enabled);
        assert_eq!(left.auto_zoom.as_ref().unwrap().hold_seconds, 4.2);
        let restored: crate::TimelineSegment =
            serde_json::from_slice(&serde_json::to_vec(&right).unwrap()).unwrap();
        assert_eq!(restored.auto_zoom, right.auto_zoom);
        let events = moves(&[(0.0, 0.5, 0.5)]);
        let crop = global.for_clip(Some(&left)).zoom_crop(false, &events, 1.0);
        assert!((crop[2] - crop[0] - 1.0 / 1.25).abs() < 0.0001);
        assert_eq!(
            global.for_clip(Some(&right)).zoom_crop(false, &events, 4.0),
            [0.0, 0.0, 1.0, 1.0]
        );
        assert_eq!(global.auto_zoom.amount, 1.75);
        let legacy: crate::TimelineSegment =
            serde_json::from_str(r#"{"start":0,"end":6,"timescale":1}"#).unwrap();
        assert_eq!(global.for_clip(Some(&legacy)).auto_zoom, global.auto_zoom);
    }
}
