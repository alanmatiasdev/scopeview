//! Camera controls (brightness, exposure, ...) as a backend-independent list.
//!
//! The mapping from the capture backend's control list is pure so it can be tested without a
//! camera. On Linux many UVC controls come through the backend as raw V4L2 ids.

use nokhwa::utils::{
    CameraControl, ControlValueDescription, ControlValueSetter, KnownCameraControl,
    KnownCameraControlFlag,
};
use nokhwa::Camera;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ControlKind {
    Brightness,
    Contrast,
    Saturation,
    Hue,
    Sharpness,
    Gamma,
    Gain,
    WhiteBalance,
    Exposure,
    /// Power line frequency: 0 off, 1 50 Hz, 2 60 Hz, 3 automatic (where offered).
    AntiFlicker,
    BacklightCompensation,
}

impl ControlKind {
    /// Display order in the panel.
    pub const ALL: [ControlKind; 11] = [
        ControlKind::Brightness,
        ControlKind::Contrast,
        ControlKind::Saturation,
        ControlKind::Hue,
        ControlKind::Sharpness,
        ControlKind::Gamma,
        ControlKind::Gain,
        ControlKind::WhiteBalance,
        ControlKind::Exposure,
        ControlKind::AntiFlicker,
        ControlKind::BacklightCompensation,
    ];

    /// Stable name used in saved files and translation keys.
    pub fn key(self) -> &'static str {
        match self {
            ControlKind::Brightness => "brightness",
            ControlKind::Contrast => "contrast",
            ControlKind::Saturation => "saturation",
            ControlKind::Hue => "hue",
            ControlKind::Sharpness => "sharpness",
            ControlKind::Gamma => "gamma",
            ControlKind::Gain => "gain",
            ControlKind::WhiteBalance => "white-balance",
            ControlKind::Exposure => "exposure",
            ControlKind::AntiFlicker => "anti-flicker",
            ControlKind::BacklightCompensation => "backlight",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// One control the device reports, with its current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlInfo {
    pub kind: ControlKind,
    pub min: i64,
    pub max: i64,
    pub step: i64,
    pub default: i64,
    pub value: i64,
    /// `Some(is_auto)` when the device has an automatic mode for this control.
    pub auto: Option<bool>,
    /// False while the device ignores writes (for example manual exposure during auto exposure).
    pub active: bool,
}

/// A value to apply: used to restore saved settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlValue {
    pub kind: ControlKind,
    pub value: i64,
    pub auto: Option<bool>,
}

/// Requests sent to a running stream's capture thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlCommand {
    Set(ControlKind, i64),
    SetAuto(ControlKind, bool),
    /// Back to the device defaults.
    Reset,
}

// V4L2 ids that the backend reports as `Other`.
const V4L2_AUTO_EXPOSURE: u128 = 0x009a_0901;
const V4L2_EXPOSURE_ABSOLUTE: u128 = 0x009a_0902;
const V4L2_AUTO_WHITE_BALANCE: u128 = 0x0098_090c;
const V4L2_POWER_LINE_FREQUENCY: u128 = 0x0098_0918;
/// V4L2 `auto_exposure` menu: manual and aperture priority.
const EXPOSURE_MANUAL: i64 = 1;
const EXPOSURE_AUTO: i64 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoStyle {
    /// A menu where one entry is manual and another automatic.
    Menu {
        manual: i64,
        auto: i64,
    },
    Boolean,
}

#[derive(Debug, Clone, PartialEq)]
enum Role {
    Value(ControlKind),
    Auto(ControlKind, AutoStyle),
}

fn classify(id: KnownCameraControl) -> Option<Role> {
    Some(match id {
        KnownCameraControl::Brightness => Role::Value(ControlKind::Brightness),
        KnownCameraControl::Contrast => Role::Value(ControlKind::Contrast),
        KnownCameraControl::Saturation => Role::Value(ControlKind::Saturation),
        KnownCameraControl::Hue => Role::Value(ControlKind::Hue),
        KnownCameraControl::Sharpness => Role::Value(ControlKind::Sharpness),
        KnownCameraControl::Gamma => Role::Value(ControlKind::Gamma),
        KnownCameraControl::Gain => Role::Value(ControlKind::Gain),
        KnownCameraControl::WhiteBalance => Role::Value(ControlKind::WhiteBalance),
        KnownCameraControl::Exposure => Role::Value(ControlKind::Exposure),
        KnownCameraControl::BacklightComp => Role::Value(ControlKind::BacklightCompensation),
        KnownCameraControl::Other(V4L2_EXPOSURE_ABSOLUTE) => Role::Value(ControlKind::Exposure),
        KnownCameraControl::Other(V4L2_POWER_LINE_FREQUENCY) => {
            Role::Value(ControlKind::AntiFlicker)
        }
        KnownCameraControl::Other(V4L2_AUTO_EXPOSURE) => Role::Auto(
            ControlKind::Exposure,
            AutoStyle::Menu {
                manual: EXPOSURE_MANUAL,
                auto: EXPOSURE_AUTO,
            },
        ),
        KnownCameraControl::Other(V4L2_AUTO_WHITE_BALANCE) => {
            Role::Auto(ControlKind::WhiteBalance, AutoStyle::Boolean)
        }
        _ => return None,
    })
}

/// A control together with the backend ids needed to write it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Resolved {
    pub info: ControlInfo,
    pub value_id: KnownCameraControl,
    pub auto: Option<(KnownCameraControl, AutoStyle, i64)>,
}

fn int_range(desc: &ControlValueDescription) -> Option<(i64, i64, i64, i64, i64)> {
    match desc {
        ControlValueDescription::IntegerRange {
            min,
            max,
            value,
            step,
            default,
        } => Some((*min, *max, *value, (*step).max(1), *default)),
        ControlValueDescription::Integer {
            value,
            default,
            step,
        } => Some((*value, *value, *value, (*step).max(1), *default)),
        _ => None,
    }
}

/// Build the control list from what the backend reports. Controls that are read-only, unknown
/// or without a usable range are left out; a kind reported twice keeps the first.
pub(crate) fn resolve(controls: &[CameraControl]) -> Vec<Resolved> {
    // First collect automatic switches by kind, as (id, style, current auto state, default state).
    let mut autos: Vec<(ControlKind, KnownCameraControl, AutoStyle, bool)> = Vec::new();
    for c in controls {
        if let Some(Role::Auto(kind, style)) = classify(c.control()) {
            let is_auto = match (style, c.description()) {
                (AutoStyle::Boolean, ControlValueDescription::Boolean { value, .. }) => *value,
                (AutoStyle::Menu { manual, .. }, d) => {
                    int_range(d).is_some_and(|(_, _, v, _, _)| v != manual)
                }
                _ => continue,
            };
            autos.push((kind, c.control(), style, is_auto));
        }
    }

    let mut out: Vec<Resolved> = Vec::new();
    for c in controls {
        let Some(Role::Value(kind)) = classify(c.control()) else {
            continue;
        };
        if out.iter().any(|r| r.info.kind == kind)
            || c.flag().contains(&KnownCameraControlFlag::ReadOnly)
        {
            continue;
        }
        let Some((min, max, value, step, default)) = int_range(c.description()) else {
            continue;
        };
        if max <= min {
            continue;
        }
        let auto = autos.iter().find(|a| a.0 == kind);
        let auto_default = auto.map_or(0, |a| match a.2 {
            AutoStyle::Menu { auto, .. } => auto,
            AutoStyle::Boolean => 1,
        });
        out.push(Resolved {
            info: ControlInfo {
                kind,
                min,
                max,
                step,
                default,
                value,
                auto: auto.map(|a| a.3),
                active: c.active(),
            },
            value_id: c.control(),
            auto: auto.map(|a| (a.1, a.2, auto_default)),
        });
    }
    out.sort_by_key(|r| r.info.kind);
    out
}

pub(crate) fn auto_setter(style: AutoStyle, on: bool) -> ControlValueSetter {
    match style {
        AutoStyle::Boolean => ControlValueSetter::Boolean(on),
        AutoStyle::Menu { manual, auto } => {
            ControlValueSetter::Integer(if on { auto } else { manual })
        }
    }
}

/// Clamp `value` into the control's range and onto its step grid.
pub fn snap(info: &ControlInfo, value: i64) -> i64 {
    let v = value.clamp(info.min, info.max);
    info.min + (v - info.min) / info.step * info.step
}

/// Owns the control state of the open camera. Lives on the capture thread.
pub(crate) struct Controller {
    resolved: Vec<Resolved>,
    /// What the user asked for, kept so it can be applied again after a reconnect.
    desired: Vec<ControlValue>,
}

impl Controller {
    pub fn new(initial: Vec<ControlValue>) -> Self {
        Self {
            resolved: Vec::new(),
            desired: initial,
        }
    }

    fn read(&mut self, camera: &Camera) {
        // Some devices have no controls at all and the backend reports an error for them.
        self.resolved = camera
            .camera_controls()
            .map(|list| resolve(&list))
            .unwrap_or_default();
    }

    fn infos(&self) -> Vec<ControlInfo> {
        self.resolved.iter().map(|r| r.info.clone()).collect()
    }

    fn write(camera: &mut Camera, r: &Resolved, v: &ControlValue) {
        match (v.auto, r.auto) {
            (Some(true), Some((id, style, _))) => {
                let _ = camera.set_camera_control(id, auto_setter(style, true));
            }
            _ => {
                if let Some((id, style, _)) = r.auto {
                    let _ = camera.set_camera_control(id, auto_setter(style, false));
                }
                let _ = camera.set_camera_control(
                    r.value_id,
                    ControlValueSetter::Integer(snap(&r.info, v.value)),
                );
            }
        }
    }

    /// After the device (re)opens: read its controls, apply the wanted values, report the result.
    pub fn attach(&mut self, camera: &mut Camera) -> Vec<ControlInfo> {
        self.read(camera);
        let desired = self.desired.clone();
        for v in &desired {
            if let Some(r) = self.resolved.iter().find(|r| r.info.kind == v.kind) {
                Self::write(camera, r, v);
            }
        }
        self.read(camera);
        self.infos()
    }

    fn upsert(&mut self, kind: ControlKind, f: impl FnOnce(&mut ControlValue)) {
        if let Some(v) = self.desired.iter_mut().find(|v| v.kind == kind) {
            f(v);
            return;
        }
        let Some(r) = self.resolved.iter().find(|r| r.info.kind == kind) else {
            return;
        };
        let mut v = ControlValue {
            kind,
            value: r.info.value,
            auto: r.info.auto,
        };
        f(&mut v);
        self.desired.push(v);
    }

    /// Apply requests in order, then report the controls as the device now has them.
    pub fn handle(&mut self, camera: &mut Camera, commands: &[ControlCommand]) -> Vec<ControlInfo> {
        for command in commands {
            match *command {
                ControlCommand::Set(kind, value) => {
                    self.upsert(kind, |v| {
                        v.value = value;
                        if v.auto.is_some() {
                            v.auto = Some(false);
                        }
                    });
                }
                ControlCommand::SetAuto(kind, on) => self.upsert(kind, |v| v.auto = Some(on)),
                ControlCommand::Reset => {
                    for r in &self.resolved {
                        if let Some((id, style, _)) = r.auto {
                            let _ = camera.set_camera_control(id, auto_setter(style, false));
                        }
                        let _ = camera.set_camera_control(
                            r.value_id,
                            ControlValueSetter::Integer(r.info.default),
                        );
                        if let Some((id, style, _)) = r.auto {
                            let _ = camera.set_camera_control(id, auto_setter(style, true));
                        }
                    }
                    self.desired.clear();
                    continue;
                }
            }
            let kind = match *command {
                ControlCommand::Set(k, _) | ControlCommand::SetAuto(k, _) => k,
                ControlCommand::Reset => continue,
            };
            let wanted = self.desired.iter().find(|v| v.kind == kind).copied();
            if let (Some(v), Some(r)) = (wanted, self.resolved.iter().find(|r| r.info.kind == kind))
            {
                Self::write(camera, r, &v);
            }
        }
        self.read(camera);
        self.infos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(min: i64, max: i64, value: i64, default: i64) -> ControlValueDescription {
        ControlValueDescription::IntegerRange {
            min,
            max,
            value,
            step: 1,
            default,
        }
    }

    fn control(
        id: KnownCameraControl,
        desc: ControlValueDescription,
        flags: Vec<KnownCameraControlFlag>,
        active: bool,
    ) -> CameraControl {
        CameraControl::new(id, "x".into(), desc, flags, active)
    }

    #[test]
    fn standard_controls_map_to_kinds_in_panel_order() {
        let list = [
            control(
                KnownCameraControl::Contrast,
                range(0, 100, 50, 50),
                vec![],
                true,
            ),
            control(
                KnownCameraControl::Brightness,
                range(-64, 64, 10, 0),
                vec![],
                true,
            ),
        ];
        let r = resolve(&list);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].info.kind, ControlKind::Brightness);
        assert_eq!(r[0].info.value, 10);
        assert_eq!(r[0].info.default, 0);
        assert_eq!(r[1].info.kind, ControlKind::Contrast);
        assert_eq!(r[0].info.auto, None);
    }

    #[test]
    fn exposure_pairs_with_the_auto_exposure_menu() {
        let list = [
            control(
                KnownCameraControl::Other(V4L2_EXPOSURE_ABSOLUTE),
                range(1, 5000, 156, 156),
                vec![],
                false,
            ),
            control(
                KnownCameraControl::Other(V4L2_AUTO_EXPOSURE),
                range(0, 3, 3, 3),
                vec![],
                true,
            ),
        ];
        let r = resolve(&list);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].info.kind, ControlKind::Exposure);
        assert_eq!(r[0].info.auto, Some(true));
        assert!(!r[0].info.active);
        let (_, style, default) = r[0].auto.unwrap();
        assert_eq!(default, 3);
        assert_eq!(auto_setter(style, false), ControlValueSetter::Integer(1));
        assert_eq!(auto_setter(style, true), ControlValueSetter::Integer(3));
    }

    #[test]
    fn manual_exposure_menu_value_means_not_auto() {
        let list = [
            control(
                KnownCameraControl::Other(V4L2_EXPOSURE_ABSOLUTE),
                range(1, 5000, 300, 156),
                vec![],
                true,
            ),
            control(
                KnownCameraControl::Other(V4L2_AUTO_EXPOSURE),
                range(0, 3, 1, 3),
                vec![],
                true,
            ),
        ];
        assert_eq!(resolve(&list)[0].info.auto, Some(false));
    }

    #[test]
    fn white_balance_uses_a_boolean_auto_switch() {
        let list = [
            control(
                KnownCameraControl::WhiteBalance,
                range(2800, 6500, 4600, 4600),
                vec![],
                true,
            ),
            control(
                KnownCameraControl::Other(V4L2_AUTO_WHITE_BALANCE),
                ControlValueDescription::Boolean {
                    value: false,
                    default: true,
                },
                vec![],
                true,
            ),
        ];
        let r = resolve(&list);
        assert_eq!(r[0].info.auto, Some(false));
        assert_eq!(
            auto_setter(r[0].auto.unwrap().1, true),
            ControlValueSetter::Boolean(true)
        );
    }

    #[test]
    fn anti_flicker_and_backlight_are_recognized() {
        let list = [
            control(
                KnownCameraControl::Other(V4L2_POWER_LINE_FREQUENCY),
                range(0, 2, 2, 1),
                vec![],
                true,
            ),
            control(
                KnownCameraControl::BacklightComp,
                range(0, 2, 1, 1),
                vec![],
                true,
            ),
        ];
        let kinds: Vec<_> = resolve(&list).iter().map(|r| r.info.kind).collect();
        assert_eq!(
            kinds,
            [ControlKind::AntiFlicker, ControlKind::BacklightCompensation]
        );
    }

    #[test]
    fn unknown_readonly_and_empty_range_controls_are_skipped() {
        let list = [
            control(
                KnownCameraControl::Other(0x1234),
                range(0, 10, 1, 1),
                vec![],
                true,
            ),
            control(
                KnownCameraControl::Gamma,
                range(0, 10, 1, 1),
                vec![KnownCameraControlFlag::ReadOnly],
                true,
            ),
            control(KnownCameraControl::Hue, range(5, 5, 5, 5), vec![], true),
            control(KnownCameraControl::Pan, range(0, 10, 1, 1), vec![], true),
        ];
        assert!(resolve(&list).is_empty());
    }

    #[test]
    fn keys_round_trip() {
        for kind in ControlKind::ALL {
            assert_eq!(ControlKind::from_key(kind.key()), Some(kind));
        }
        assert_eq!(ControlKind::from_key("nope"), None);
    }

    #[test]
    fn snap_clamps_and_follows_the_step() {
        let info = ControlInfo {
            kind: ControlKind::Gain,
            min: 10,
            max: 100,
            step: 5,
            default: 10,
            value: 10,
            auto: None,
            active: true,
        };
        assert_eq!(snap(&info, 3), 10);
        assert_eq!(snap(&info, 999), 100);
        assert_eq!(snap(&info, 47), 45);
    }
}
