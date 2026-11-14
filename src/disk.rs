#[derive(Clone, Debug)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub fs_type: String,
    pub total_bytes: u64,
    pub avail_bytes: u64,
}

impl DiskInfo {
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.avail_bytes)
    }
    pub fn usage_ratio(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.used_bytes() as f32 / self.total_bytes as f32
        }
    }
    pub fn total_gb(&self) -> f64 {
        self.total_bytes as f64 / 1_073_741_824.0
    }
    pub fn used_gb(&self) -> f64 {
        self.used_bytes() as f64 / 1_073_741_824.0
    }
    pub fn avail_gb(&self) -> f64 {
        self.avail_bytes as f64 / 1_073_741_824.0
    }
    pub fn total_mb(&self) -> f64 {
        self.total_bytes as f64 / 1_048_576.0
    }
    pub fn used_mb(&self) -> f64 {
        self.used_bytes() as f64 / 1_048_576.0
    }
    pub fn avail_mb(&self) -> f64 {
        self.avail_bytes as f64 / 1_048_576.0
    }
}

pub fn get_disks() -> Vec<DiskInfo> {
    use sysinfo::Disks;
    let mut disks: Vec<DiskInfo> = Disks::new_with_refreshed_list()
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| DiskInfo {
            name: d.name().to_string_lossy().into_owned(),
            mount: d.mount_point().to_string_lossy().into_owned(),
            fs_type: d.file_system().to_string_lossy().into_owned(),
            total_bytes: d.total_space(),
            avail_bytes: d.available_space(),
        })
        .collect();

    // Sort so primary root (/) or larger drives come first
    disks.sort_by(|a, b| {
        let a_root = a.mount == "/";
        let b_root = b.mount == "/";
        b_root.cmp(&a_root).then_with(|| b.total_bytes.cmp(&a.total_bytes))
    });
    disks
}
