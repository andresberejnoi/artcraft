//! WebKitGTK + NVIDIA compatibility workarounds.
//!
//! WebKitGTK 2.44+ (verified against 2.54) cannot satisfy the strict Wayland
//! explicit-sync handshake demanded by NVIDIA's proprietary driver (verified
//! against 615.71.09). The failure is startup-level and severe: either a
//! `Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.`
//! crash before the first frame, or broken frame presentation (e.g. an HTML5
//! `<video>` painting above all DOM, making the login screen unusable).
//! Disabling NVIDIA's explicit-sync handshake lets WebKitGTK's native
//! DMABUF/compositing pipeline present frames correctly at full acceleration.
//!
//! This must run before the webview is created and therefore before the Tauri
//! logger is installed, so it stays silent — it is an environment default, not
//! an event.

/// NVIDIA's proprietary kernel module registers this directory when loaded.
/// Its presence reliably signals that the proprietary driver is in use.
#[cfg(target_os = "linux")]
const NVIDIA_KERNEL_MODULE_PATH: &str = "/sys/module/nvidia";

/// The driver also reports its version through procfs. This is a fallback for
/// environments that do not mount `/sys/module`.
#[cfg(target_os = "linux")]
const NVIDIA_PROC_VERSION_PATH: &str = "/proc/driver/nvidia/version";

/// Env var understood by NVIDIA's proprietary driver to skip the strict Wayland
/// explicit-sync handshake that WebKitGTK cannot complete.
#[cfg(target_os = "linux")]
const NVIDIA_DISABLE_EXPLICIT_SYNC_ENV: &str = "__NV_DISABLE_EXPLICIT_SYNC";

#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::ffi::OsStr;
#[cfg(target_os = "linux")]
use std::path::Path;

/// Applies the NVIDIA explicit-sync workaround when the proprietary driver is
/// in use. Deliberately does not touch `WEBKIT_DISABLE_*`; those remain the
/// user's or launcher's explicit choice.
#[cfg(target_os = "linux")]
pub fn apply_webkit_nvidia_workarounds() {
  let nvidia_driver_present = Path::new(NVIDIA_KERNEL_MODULE_PATH).exists()
    || Path::new(NVIDIA_PROC_VERSION_PATH).exists();
  let explicit_sync = env::var_os(NVIDIA_DISABLE_EXPLICIT_SYNC_ENV);

  if !should_set_explicit_sync(nvidia_driver_present, explicit_sync.as_deref()) {
    return;
  }

  // Safe: this runs single-threaded at the very top of `run()`, before the
  // Tauri runtime spawns any threads or initializes the webview.
  env::set_var(NVIDIA_DISABLE_EXPLICIT_SYNC_ENV, "1");
}

/// Returns whether the explicit-sync default should be applied. An existing
/// value always wins, including `0` and an empty value.
#[cfg(target_os = "linux")]
fn should_set_explicit_sync(
  nvidia_driver_present: bool,
  explicit_sync: Option<&OsStr>,
) -> bool {
  nvidia_driver_present && explicit_sync.is_none()
}

#[cfg(not(target_os = "linux"))]
pub fn apply_webkit_nvidia_workarounds() {}

#[cfg(all(test, target_os = "linux"))]
mod tests {
  use super::*;

  #[test]
  fn does_nothing_without_nvidia() {
    assert!(!should_set_explicit_sync(false, None));
    assert!(!should_set_explicit_sync(false, Some(OsStr::new("0"))));
  }

  #[test]
  fn sets_explicit_sync_when_nvidia_and_unset() {
    assert!(should_set_explicit_sync(true, None));
  }

  #[test]
  fn respects_existing_explicit_sync_value() {
    assert!(!should_set_explicit_sync(true, Some(OsStr::new("0"))));
    assert!(!should_set_explicit_sync(true, Some(OsStr::new("1"))));
    assert!(!should_set_explicit_sync(true, Some(OsStr::new(""))));
  }
}
