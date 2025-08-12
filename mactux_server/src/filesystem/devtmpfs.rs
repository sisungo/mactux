use crate::{
    filesystem::{kernfs::{DirEntry, KernFs}, vfs::Mountable},
    util::FileAttrs,
};
use std::sync::Arc;
use structures::error::LxError;

pub fn mountable() -> Result<Arc<dyn Mountable>, LxError> {
    let kernfs = KernFs::new();
    let mut writer = kernfs.0.table.write().unwrap();

    #[cfg(feature = "audio")]
    {
        writer.insert(
            "dsp".into(),
            DirEntry::RegularFile(Arc::new(crate::audio::oss::OssDevice::new(
                FileAttrs::common(),
            ))),
        );
    }

    writer.insert(
        "null".into(),
        DirEntry::RegularFile(Arc::new(crate::device::Null)),
    );
    writer.insert(
        "zero".into(),
        DirEntry::RegularFile(Arc::new(crate::device::Zero)),
    );
    writer.insert(
        "random".into(),
        DirEntry::RegularFile(Arc::new(crate::device::Random)),
    );
    writer.insert(
        "urandom".into(),
        DirEntry::RegularFile(Arc::new(crate::device::URandom)),
    );

    drop(writer);
    Ok(Arc::new(kernfs))
}
