//! Pure selection rules for the device list, kept free of UI and camera code so they are testable.

use scopeview_camera::DeviceInfo;

/// After refreshing the device list, keep the user's device if it is still there (matched by
/// name, since indexes can change), otherwise fall back to the first device.
pub fn pick_device(devices: &[DeviceInfo], previous_name: Option<&str>) -> Option<usize> {
    if devices.is_empty() {
        return None;
    }
    previous_name
        .and_then(|name| devices.iter().position(|d| d.name == name))
        .or(Some(0))
}

/// True when the lists differ in a way the user can see (names or order), ignoring index churn.
pub fn device_list_changed(old: &[DeviceInfo], new: &[DeviceInfo]) -> bool {
    old.len() != new.len() || old.iter().zip(new).any(|(a, b)| a.name != b.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scopeview_camera::DeviceId;

    fn dev(id: u32, name: &str) -> DeviceInfo {
        DeviceInfo {
            id: DeviceId(id),
            name: name.to_string(),
        }
    }

    #[test]
    fn empty_list_selects_nothing() {
        assert_eq!(pick_device(&[], Some("Cam")), None);
    }

    #[test]
    fn keeps_previous_device_by_name() {
        let devices = [dev(0, "A"), dev(1, "B")];
        assert_eq!(pick_device(&devices, Some("B")), Some(1));
    }

    #[test]
    fn falls_back_to_first_when_previous_is_gone() {
        let devices = [dev(0, "A"), dev(1, "B")];
        assert_eq!(pick_device(&devices, Some("C")), Some(0));
        assert_eq!(pick_device(&devices, None), Some(0));
    }

    #[test]
    fn index_churn_is_not_a_change() {
        assert!(!device_list_changed(&[dev(0, "A")], &[dev(3, "A")]));
    }

    #[test]
    fn added_or_renamed_devices_are_changes() {
        assert!(device_list_changed(&[dev(0, "A")], &[]));
        assert!(device_list_changed(&[dev(0, "A")], &[dev(0, "B")]));
    }
}
