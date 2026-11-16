//! Container and virtual machine evidence, combined into the environment class.

use crate::profile::Environment;

/// Does `/proc/cpuinfo` carry the `hypervisor` flag? Only the flag lines are inspected.
#[must_use]
pub fn cpuinfo_hypervisor(text: &str) -> bool {
    text.lines()
        .filter(|l| l.starts_with("flags"))
        .any(|l| l.split_whitespace().any(|w| w == "hypervisor"))
}

/// Container runtime keywords in `/proc/self/cgroup`.
#[must_use]
pub fn cgroup_container(text: &str) -> bool {
    const WORDS: [&str; 6] = ["docker", "kubepods", "lxc", "containerd", "libpod", "crio"];
    text.lines().any(|l| {
        let l = l.to_ascii_lowercase();
        WORDS.iter().any(|w| l.contains(w))
    })
}

/// DMI vendor or product text names a hypervisor (the text itself is dropped).
#[must_use]
pub fn dmi_virtual(text: &str) -> bool {
    const WORDS: [&str; 9] = [
        "qemu",
        "kvm",
        "vmware",
        "virtualbox",
        "xen",
        "bochs",
        "parallels",
        "hyper-v",
        "virtual machine",
    ];
    let l = text.to_ascii_lowercase();
    WORDS.iter().any(|w| l.contains(w))
}

/// Combines the evidence into the environment class (container wins over VM).
#[must_use]
pub fn environment(container: bool, vm: bool) -> Environment {
    if container {
        Environment::Container
    } else if vm {
        Environment::Vm
    } else {
        Environment::Bare
    }
}
