// expect: reject E_CAPABILITY
// pending: Narrow external-child execution deliberately requires an immutable
// authenticated admitted image, checked entry grants and transferred handles.
// An exact path alone is insufficient; the diagnostic names this proof and
// World.process() as explicitly broader host authority.
// A future executable test must deny reading an outside fixture and observing
// an unrelated inherited descriptor even when the host user could access them.
use std::process::Command;
pub fn probe() {
    Command::new("/usr/bin/cat").arg("outside-parent-directory").output();
}
