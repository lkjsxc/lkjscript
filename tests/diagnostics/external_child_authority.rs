// expect: reject E_CAPABILITY
// pending: An exact external executable path does not prove its child effects
// stay within this entry's grants; name the missing admitted-child proof or
// explicitly broader host authority, not merely "processes not implemented".
// A future executable test must deny reading an outside fixture and observing
// an unrelated inherited descriptor even when the host user could access them.
use std::process::Command;
pub fn probe() {
    Command::new("/usr/bin/cat").arg("outside-parent-directory").output();
}
