use std::fs;
use std::path::{Path, PathBuf};

pub struct Fixture {
    root: PathBuf,
}

impl Fixture {
    pub fn new(cpus: &[u32], online: &str, allowed: &str) -> std::io::Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "lkjscript-linux-host-{}-{}",
            std::process::id(),
            unique()
        ));
        fs::create_dir_all(&root)?;
        let fixture = Self { root };
        fixture.write("sys/devices/system/cpu/online", online)?;
        for cpu in cpus {
            fixture.cpu(*cpu, 0, 0, *cpu, &cpu.to_string())?;
        }
        fixture.status("proc/self/status", allowed, allowed)?;
        fixture.status("proc/thread-self/status", allowed, allowed)?;
        fixture.write("proc/self/sched", "fixture (1, #threads: 1)\npolicy : 0\n")?;
        fixture.write("proc/sys/kernel/osrelease", "fixture-kernel")?;
        fixture.write("proc/sys/kernel/numa_balancing", "0")?;
        fixture.write("proc/self/cgroup", "0::/fixture")?;
        fixture.write("sys/fs/cgroup/fixture/cpuset.cpus.effective", allowed)?;
        fixture.write("sys/fs/cgroup/fixture/cpu.max", "max 100000")?;
        Ok(fixture)
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn write(&self, relative: &str, value: &str) -> std::io::Result<()> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, value)
    }

    pub fn remove(&self, relative: &str) -> std::io::Result<()> {
        fs::remove_file(self.root.join(relative))
    }

    pub fn cpu(
        &self,
        cpu: u32,
        package: u32,
        die: u32,
        core: u32,
        siblings: &str,
    ) -> std::io::Result<()> {
        let base = format!("sys/devices/system/cpu/cpu{cpu}/topology");
        self.write(&format!("{base}/physical_package_id"), &package.to_string())?;
        self.write(&format!("{base}/die_id"), &die.to_string())?;
        self.write(&format!("{base}/core_id"), &core.to_string())?;
        self.write(&format!("{base}/thread_siblings_list"), siblings)
    }

    pub fn cache(
        &self,
        cpu: u32,
        index: u32,
        id: u32,
        level: u8,
        kind: &str,
        shared: &str,
    ) -> std::io::Result<()> {
        let base = format!("sys/devices/system/cpu/cpu{cpu}/cache/index{index}");
        self.write(&format!("{base}/id"), &id.to_string())?;
        self.write(&format!("{base}/level"), &level.to_string())?;
        self.write(&format!("{base}/type"), kind)?;
        self.write(&format!("{base}/shared_cpu_list"), shared)?;
        self.write(&format!("{base}/size"), "8M")?;
        self.write(&format!("{base}/coherency_line_size"), "64")?;
        self.write(&format!("{base}/ways_of_associativity"), "16")
    }

    pub fn node(&self, id: u32, cpus: &str, distances: &str, kib: u64) -> std::io::Result<()> {
        let base = format!("sys/devices/system/node/node{id}");
        self.write(&format!("{base}/cpulist"), cpus)?;
        self.write(&format!("{base}/distance"), distances)?;
        self.write(
            &format!("{base}/meminfo"),
            &format!("Node {id} MemTotal: {kib} kB\n"),
        )
    }

    pub fn status(&self, path: &str, list: &str, map_list: &str) -> std::io::Result<()> {
        let map = cpu_map(map_list)?;
        self.write(
            path,
            &format!("Name:\tfixture\nCpus_allowed:\t{map}\nCpus_allowed_list:\t{list}\n"),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn cpu_map(list: &str) -> std::io::Result<String> {
    let set = lkjscript_resource::CpuSet::parse_list(list)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let highest = *set
        .as_slice()
        .last()
        .ok_or_else(|| std::io::Error::other("empty CPU list"))? as usize;
    let mut words = vec![0_u32; highest / 32 + 1];
    for cpu in set.as_slice() {
        words[*cpu as usize / 32] |= 1 << (*cpu as usize % 32);
    }
    Ok(words
        .iter()
        .rev()
        .map(|word| format!("{word:08x}"))
        .collect::<Vec<_>>()
        .join(","))
}

fn unique() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}
