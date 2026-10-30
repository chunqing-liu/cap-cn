use crate::*;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::borrow::Cow;

#[derive(Type, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ClipSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<BackgroundConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<Camera>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<CursorConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_correction: Option<ColorCorrectionConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_motion_blur: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption_settings: Option<CaptionSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyboard: Option<KeyboardData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dual_display: Option<DualDisplayConfiguration>,
}

impl ClipSettings {
    pub fn apply(&self, project: &mut ProjectConfiguration) {
        macro_rules! apply {
            ($($field:ident),*) => {$(
                if let Some(value) = &self.$field {
                    project.$field = value.clone();
                }
            )*};
        }
        apply!(background, camera, cursor, audio, color_correction);
        if let Some(value) = self.screen_motion_blur {
            project.screen_motion_blur = value;
        }
        if let Some(value) = &self.caption_settings {
            project
                .captions
                .get_or_insert_with(Default::default)
                .settings = value.clone();
        }
        if let Some(value) = &self.keyboard {
            project.keyboard = Some(value.clone());
        }
        if let (Some(value), Some(base)) = (&self.dual_display, &mut project.dual_display) {
            let canvas_size = base.canvas_size;
            *base = value.clone();
            base.canvas_size = canvas_size;
        }
    }
}

impl ProjectConfiguration {
    pub fn clip_settings_at(&self, time: f64) -> Option<&ClipSettings> {
        self.get_segment_time(time)?.1.settings.as_ref()
    }

    pub fn clip_control_project(&self, index: usize) -> Cow<'_, Self> {
        let mut project = Cow::Borrowed(self);
        if let Some(segment) = self.timeline.as_ref().and_then(|t| t.segments.get(index)) {
            if let Some(settings) = &segment.settings {
                settings.apply(project.to_mut());
            }
            if segment.hide_cursor == Some(true) {
                project.to_mut().cursor.hide = true;
            }
            if let (Some(zoom), Some(_)) = (&segment.auto_zoom, &self.dual_display) {
                project.to_mut().dual_display.as_mut().unwrap().auto_zoom = zoom.clone();
            }
        }
        project
    }

    pub fn clip_audio_project(&self, segment: &TimelineSegment) -> Cow<'_, Self> {
        match segment
            .settings
            .as_ref()
            .and_then(|settings| settings.audio.as_ref())
        {
            Some(audio) => {
                let mut project = self.clone();
                project.audio = audio.clone();
                Cow::Owned(project)
            }
            None => Cow::Borrowed(self),
        }
    }
}

fn changed_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
    target: &mut serde_json::Value,
) {
    if before == after {
        return;
    }
    if let (Some(before), Some(after), Some(target)) = (
        before.as_object(),
        after.as_object(),
        target.as_object_mut(),
    ) {
        for (key, value) in after {
            let old = before.get(key).unwrap_or(&serde_json::Value::Null);
            if old != value {
                let target = target.entry(key.clone()).or_insert(serde_json::Value::Null);
                changed_fields(old, value, target);
            }
        }
        for key in before.keys().filter(|key| !after.contains_key(*key)) {
            target.remove(key);
        }
    } else {
        *target = after.clone();
    }
}

pub fn apply_clip_control_change(
    project: &mut ProjectConfiguration,
    indices: &[usize],
    mut change: impl FnMut(&mut ProjectConfiguration) -> bool,
) -> Result<bool, serde_json::Error> {
    let Some(&first) = indices.first() else {
        return Ok(false);
    };
    let Some(timeline) = &project.timeline else {
        return Ok(false);
    };
    if indices
        .iter()
        .any(|&index| index >= timeline.segments.len())
    {
        return Ok(false);
    }
    let before = project.clip_control_project(first).into_owned();
    let mut edited = before.clone();
    let mut changed = change(&mut edited);
    let mut updates = Vec::new();
    for &index in indices {
        let effective = project.clip_control_project(index);
        let before = effective.as_ref();
        let mut per_clip = before.clone();
        if index == first {
            per_clip = edited.clone();
        } else {
            changed |= change(&mut per_clip);
        }
        let edited = &per_clip;
        let mut settings = project.timeline.as_ref().unwrap().segments[index]
            .settings
            .clone()
            .unwrap_or_default();
        macro_rules! patch {
            ($($field:ident),*) => {$(
                let old = serde_json::to_value(&before.$field)?;
                let new = serde_json::to_value(&edited.$field)?;
                if old != new {
                    let mut target = serde_json::to_value(&effective.$field)?;
                    changed_fields(&old, &new, &mut target);
                    settings.$field = Some(serde_json::from_value(target)?);
                }
            )*};
        }
        patch!(
            background,
            camera,
            cursor,
            audio,
            color_correction,
            screen_motion_blur
        );
        let old = before
            .captions
            .as_ref()
            .map(|v| &v.settings)
            .cloned()
            .unwrap_or_default();
        let new = edited
            .captions
            .as_ref()
            .map(|v| &v.settings)
            .cloned()
            .unwrap_or_default();
        let mut target = serde_json::to_value(
            effective
                .captions
                .as_ref()
                .map(|v| &v.settings)
                .cloned()
                .unwrap_or_default(),
        )?;
        if serde_json::to_value(&old)? != serde_json::to_value(&new)? {
            changed_fields(
                &serde_json::to_value(old)?,
                &serde_json::to_value(new)?,
                &mut target,
            );
            settings.caption_settings = Some(serde_json::from_value(target)?);
        }
        if serde_json::to_value(&before.keyboard)? != serde_json::to_value(&edited.keyboard)? {
            let mut target = serde_json::to_value(effective.keyboard.clone().unwrap_or_default())?;
            changed_fields(
                &serde_json::to_value(before.keyboard.clone().unwrap_or_default())?,
                &serde_json::to_value(edited.keyboard.clone().unwrap_or_default())?,
                &mut target,
            );
            settings.keyboard = Some(serde_json::from_value(target)?);
        }
        let mut zoom = None;
        if let (Some(old), Some(new), Some(base)) = (
            &before.dual_display,
            &edited.dual_display,
            &effective.dual_display,
        ) {
            let mut new = new.clone();
            new.canvas_size = old.canvas_size;
            if old != &new {
                let mut target = serde_json::to_value(base)?;
                changed_fields(
                    &serde_json::to_value(old)?,
                    &serde_json::to_value(&new)?,
                    &mut target,
                );
                let dual: DualDisplayConfiguration = serde_json::from_value(target)?;
                if old.auto_zoom != new.auto_zoom {
                    zoom = Some(dual.auto_zoom.clone());
                }
                settings.dual_display = Some(dual);
            }
        }
        let previous = project.timeline.as_ref().unwrap().segments[index]
            .settings
            .clone()
            .unwrap_or_default();
        if serde_json::to_value(&previous)? != serde_json::to_value(&settings)? || zoom.is_some() {
            updates.push((
                index,
                settings,
                zoom,
                before.cursor.hide != edited.cursor.hide,
            ));
        }
    }
    if !changed {
        return Ok(false);
    }
    // Preserve timeline operations and project-wide canvas changes, but never publish the temporary control snapshot.
    edited.background = project.background.clone();
    edited.camera = project.camera.clone();
    edited.cursor = project.cursor.clone();
    edited.audio = project.audio.clone();
    edited.color_correction = project.color_correction.clone();
    edited.screen_motion_blur = project.screen_motion_blur;
    edited.keyboard = project.keyboard.clone();
    if let Some(captions) = &mut edited.captions {
        captions.settings = project
            .captions
            .as_ref()
            .map(|v| v.settings.clone())
            .unwrap_or_default();
    }
    let canvas = edited.dual_display.as_ref().map(|v| v.canvas_size);
    edited.dual_display = project.dual_display.clone();
    if let (Some(dual), Some(canvas)) = (&mut edited.dual_display, canvas) {
        dual.canvas_size = canvas;
    }
    if let Some(timeline) = &mut edited.timeline {
        for (index, settings, zoom, cursor_visibility_changed) in updates {
            if let Some(segment) = timeline.segments.get_mut(index) {
                segment.settings = Some(settings);
                if cursor_visibility_changed {
                    segment.hide_cursor = None;
                }
                if let Some(zoom) = zoom {
                    segment.auto_zoom = Some(zoom);
                }
            }
        }
    }
    *project = edited;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> ProjectConfiguration {
        serde_json::from_value(json!({
            "timeline": { "segments": [
                {"recordingSegment":0,"start":0.0,"end":5.0,"timescale":1.0},
                {"recordingSegment":0,"start":5.0,"end":10.0,"timescale":1.0},
                {"recordingSegment":0,"start":10.0,"end":15.0,"timescale":1.0}
            ], "zoomSegments": []}
        }))
        .unwrap()
    }

    #[test]
    fn legacy_defaults_and_cursor_settings_are_clip_local() {
        let mut project = fixture();
        assert!(
            project.timeline.as_ref().unwrap().segments[0]
                .settings
                .is_none()
        );
        let original = serde_json::to_value(&project.cursor).unwrap();
        apply_clip_control_change(&mut project, &[1], |p| {
            p.cursor.hide_when_idle = true;
            p.cursor.hide_when_idle_delay = 0.8;
            p.cursor.size = 150;
            true
        })
        .unwrap();
        assert_eq!(serde_json::to_value(&project.cursor).unwrap(), original);
        assert!(!project.cursor_at(4.99).hide_when_idle);
        assert!(project.cursor_at(5.0).hide_when_idle);
        assert_eq!(project.cursor_at(6.0).size, 150);
        assert!(!project.cursor_at(10.0).hide_when_idle);
        let restored: ProjectConfiguration =
            serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
        assert!(restored.cursor_at(7.0).hide_when_idle);
        assert!(!restored.cursor_at(11.0).hide_when_idle);
    }

    #[test]
    fn multiselect_patches_only_changed_fields_and_preserves_other_clips() {
        let mut project = fixture();
        apply_clip_control_change(&mut project, &[1], |p| {
            p.cursor.size = 175;
            p.cursor.ripple.enabled = true;
            true
        })
        .unwrap();
        apply_clip_control_change(&mut project, &[0, 1], |p| {
            p.cursor.hide_when_idle = true;
            true
        })
        .unwrap();
        assert!(project.cursor_at(1.0).hide_when_idle);
        assert!(project.cursor_at(6.0).hide_when_idle);
        assert_eq!(project.cursor_at(6.0).size, 175);
        assert!(project.cursor_at(6.0).ripple.enabled);
        assert_eq!(project.cursor_at(1.0).size, project.cursor.size);
        assert!(!project.cursor_at(11.0).hide_when_idle);
    }

    #[test]
    fn multiselect_applies_value_even_when_first_clip_already_has_it() {
        let mut project = fixture();
        apply_clip_control_change(&mut project, &[1], |p| {
            p.cursor.size = 175;
            true
        })
        .unwrap();
        let desired = project.cursor.size;
        assert!(
            apply_clip_control_change(&mut project, &[0, 1], |p| {
                if p.cursor.size == desired {
                    return false;
                }
                p.cursor.size = desired;
                true
            })
            .unwrap()
        );
        assert_eq!(project.cursor_at(6.0).size, desired);
    }

    #[test]
    fn split_copy_reorder_and_speed_keep_settings_attached_to_clip() {
        let mut project = fixture();
        apply_clip_control_change(&mut project, &[0], |p| {
            p.cursor.hide = true;
            true
        })
        .unwrap();
        let timeline = project.timeline.as_mut().unwrap();
        let mut tail = timeline.segments[0].clone();
        tail.start = 2.0;
        timeline.segments[0].end = 2.0;
        timeline.segments.insert(1, tail);
        apply_clip_control_change(&mut project, &[1], |p| {
            p.cursor.hide = false;
            true
        })
        .unwrap();
        assert!(project.cursor_at(1.0).hide);
        assert!(!project.cursor_at(3.0).hide);
        let timeline = project.timeline.as_mut().unwrap();
        timeline.segments.swap(0, 2);
        timeline.segments[0].timescale = 100.0;
        assert!(!project.cursor_at(0.01).hide);
        assert!(project.cursor_at(3.1).hide);
    }

    #[test]
    fn visual_audio_and_dual_settings_resolve_without_changing_output_canvas() {
        let mut project = fixture();
        project.dual_display = Some(Default::default());
        let base_canvas = project.dual_display.as_ref().unwrap().canvas_size;
        apply_clip_control_change(&mut project, &[1], |p| {
            p.background.rounding = 42.0;
            p.camera.hide = true;
            p.audio.mute = true;
            p.color_correction.screen.exposure = 0.7;
            p.screen_motion_blur = 0.2;
            p.dual_display.as_mut().unwrap().auto_zoom.amount = 2.5;
            true
        })
        .unwrap();
        let resolved = project.style_at(6.0);
        assert_eq!(resolved.background.rounding, 42.0);
        assert!(resolved.camera.hide);
        assert!(resolved.audio.mute);
        assert_eq!(resolved.color_correction.screen.exposure, 0.7);
        assert_eq!(resolved.screen_motion_blur, 0.2);
        assert_eq!(
            resolved.dual_display.as_ref().unwrap().canvas_size,
            base_canvas
        );
        assert_eq!(
            project.timeline.as_ref().unwrap().segments[1]
                .auto_zoom
                .as_ref()
                .unwrap()
                .amount,
            2.5
        );
        assert!(!project.audio.mute);
        assert!(!project.style_at(1.0).audio.mute);
        let clip = &project.timeline.as_ref().unwrap().segments[1];
        assert!(project.clip_audio_project(clip).audio.mute);
    }

    #[test]
    fn structural_edits_do_not_add_or_retarget_settings() {
        let mut project = fixture();
        apply_clip_control_change(&mut project, &[1], |p| {
            p.timeline.as_mut().unwrap().segments.remove(0);
            true
        })
        .unwrap();
        assert_eq!(project.timeline.as_ref().unwrap().segments.len(), 2);
        assert!(
            project
                .timeline
                .as_ref()
                .unwrap()
                .segments
                .iter()
                .all(|s| s.settings.is_none())
        );
        let original = serde_json::to_value(&project).unwrap();
        assert!(
            !apply_clip_control_change(&mut project, &[99], |_| panic!("must not execute"))
                .unwrap()
        );
        assert_eq!(serde_json::to_value(project).unwrap(), original);
    }

    #[test]
    fn show_cursor_control_can_clear_the_legacy_clip_hide_flag() {
        let mut project = fixture();
        project.timeline.as_mut().unwrap().segments[0].hide_cursor = Some(true);
        assert!(project.clip_control_project(0).cursor.hide);
        apply_clip_control_change(&mut project, &[0], |p| {
            p.cursor.hide = false;
            true
        })
        .unwrap();
        assert_eq!(
            project.timeline.as_ref().unwrap().segments[0].hide_cursor,
            None
        );
        assert!(!project.clip_control_project(0).cursor.hide);
    }
}
