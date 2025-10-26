mod auxv;
mod mmap;
mod stack;

use crate::{Error, IoFd};
use auxv::AuxiliaryInfo;
use mmap::*;
use object::{
    LittleEndian, ReadCache,
    elf::{PT_INTERP, PT_LOAD, ProgramHeader64},
    read::elf::{ElfFile64, FileHeader, ProgramHeader},
};
use rand::RngCore;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};
use structures::{error::LxError, fs::OpenFlags};

type ExecutableObject<'a> = ElfFile64<'a, LittleEndian, &'a ReadCache<IoFd<'a>>>;

/// A loaded Linux program.
#[derive(Debug)]
pub struct Program {
    exec_fd: OwnedFd,

    interpreter: Option<Box<Self>>,
    phdr: *const u8,
    phent: usize,
    phnum: usize,
    entry: *const u8,

    base_map: MappedArea,
    _mapped_areas: Vec<MappedArea>,
}
impl Program {
    pub const MAGIC: &[u8] = &[0x7f, 0x45, 0x4c, 0x46];

    /// Loads a Linux program from the given file descriptor.
    pub fn load(exec_fd: OwnedFd) -> Result<Self, Error> {
        let read_cache = ReadCache::new(IoFd(exec_fd.as_fd()));
        let main =
            ExecutableObject::parse(&read_cache).map_err(|x| Error::ImageFormat(x.to_string()))?;
        let mut interpreter = None;
        let base_map = map_base(&main)?;
        let entry = unsafe {
            base_map
                .addr()
                .add(main.elf_header().e_entry(LittleEndian) as usize)
        };

        let mut _mapped_areas = Vec::new();
        for phdr in main.elf_program_headers().iter() {
            match phdr.p_type(LittleEndian) {
                PT_INTERP => {
                    interpreter = Some(Box::new(Self::load(read_interp(phdr, &read_cache)?)?));
                }
                PT_LOAD => {
                    let mapped_area = map_phdr(phdr, exec_fd.as_raw_fd(), base_map.addr())
                        .map_err(|x| Error::LoadImage(x.into()))?;
                    if base_map.addr().is_null() {
                        _mapped_areas.push(mapped_area);
                    }
                }
                _ => continue,
            }
        }

        let phdr = unsafe {
            base_map
                .addr()
                .add(main.elf_header().e_phoff(LittleEndian) as usize)
        };
        let phent = main.elf_header().e_phentsize(LittleEndian) as _;
        let phnum = main.elf_header().e_phnum(LittleEndian) as _;

        Ok(Program {
            exec_fd,

            interpreter,
            phdr,
            phent,
            phnum,
            entry,

            base_map,
            _mapped_areas,
        })
    }

    /// Runs the program.
    pub unsafe fn run<'a, 'b>(&self, args: &[&[u8]], envs: &[&[u8]]) {
        let base = match &self.interpreter {
            Some(interp) => interp.base_map.addr() as usize,
            None => 0,
        };
        let entry = match &self.interpreter {
            Some(interp) => interp.entry,
            None => self.entry,
        };
        let mut random = Box::new([0u8; 64]);
        rand::rng().fill_bytes(&mut *random);
        let auxv = AuxiliaryInfo {
            exec_fd: self.exec_fd.as_raw_fd() as _,
            phdr_base: self.phdr as usize,
            phdr_size: self.phent as usize,
            phdr_count: self.phnum as usize,
            entry: self.entry as usize,
            base,
            random: Box::into_raw(random),
        };
        stack::jump(entry, args, envs, auxv);
    }
}

fn map_base(main: &ExecutableObject) -> Result<MappedArea, Error> {
    if main.elf_header().e_type.get(LittleEndian) == object::elf::ET_DYN {
        let top_phdr = main
            .elf_program_headers()
            .iter()
            .filter(|x| x.p_type(LittleEndian) == PT_LOAD)
            .max_by_key(|x| x.p_vaddr(LittleEndian))
            .ok_or_else(|| Error::ImageFormat(String::from("image has no PT_LOAD segment")))?;
        let max_addr = top_phdr.p_vaddr(LittleEndian) + top_phdr.p_memsz(LittleEndian);
        unsafe {
            MappedArea::builder()
                .len(max_addr as _)
                .build()
                .map_err(|x| Error::LoadImage(x.into()))
        }
    } else {
        Ok(MappedArea::null())
    }
}

/// Reads `PT_INTERP` from a program header.
fn read_interp(
    phdr: &ProgramHeader64<LittleEndian>,
    read_cache: &ReadCache<IoFd>,
) -> Result<OwnedFd, Error> {
    let path = phdr
        .interpreter(LittleEndian, read_cache)
        .map_err(|x| Error::ImageFormat(x.to_string()))?
        .ok_or_else(|| Error::ImageFormat(String::from("invalid PT_INTERP segment")))?;
    let interp_fd = rtenv::fs::open(path.into(), OpenFlags::O_CLOEXEC | OpenFlags::O_RDONLY, 0)
        .map_err(Error::ReadImage)?;
    if rtenv::vfd::get(interp_fd).is_some() {
        return Err(Error::ReadImage(LxError::EACCES));
    }
    unsafe { Ok(OwnedFd::from_raw_fd(interp_fd)) }
}

/// Maps a `PT_LOAD` program header to process memory.
fn map_phdr(
    phdr: &ProgramHeader64<LittleEndian>,
    fd: RawFd,
    mem_base: *mut u8,
) -> std::io::Result<MappedArea> {
    let p_filesz = phdr.p_filesz(LittleEndian);
    let p_memsz = phdr.p_memsz(LittleEndian) as usize;
    let p_vaddr = phdr.p_vaddr(LittleEndian) as usize;
    let p_offset = phdr.p_offset(LittleEndian);

    let fill_align = phdr.p_vaddr(LittleEndian) % page_size() as u64;
    let segment_base = mem_base as usize + p_vaddr - fill_align as usize;

    let mut builder = MappedArea::builder()
        .destination(segment_base)
        .len(p_memsz + fill_align as usize);
    if !mem_base.is_null() {
        builder = builder.auto_unmap(false);
    }
    elf_mmap_perms(&mut builder, phdr);
    let mapped = unsafe { builder.build()? };

    let mut builder = MappedArea::builder()
        .file(fd, p_offset - fill_align)
        .len(p_filesz as usize + fill_align as usize)
        .destination(segment_base)
        .auto_unmap(false);
    elf_mmap_perms(&mut builder, phdr);
    unsafe { builder.build()? };

    if phdr.p_flags(LittleEndian) & object::elf::PF_W != 0 {
        unsafe {
            mem_base
                .add(p_vaddr + p_filesz as usize)
                .write_bytes(0, p_memsz - p_filesz as usize);
        }
    }

    Ok(mapped)
}

fn elf_mmap_perms(builder: &mut MappedAreaBuilder, phdr: &ProgramHeader64<LittleEndian>) {
    if phdr.p_flags(LittleEndian) & object::elf::PF_R != 0 {
        *builder = builder.readable();
    }
    if phdr.p_flags(LittleEndian) & object::elf::PF_W != 0 {
        *builder = builder.writable();
    }
    if phdr.p_flags(LittleEndian) & object::elf::PF_X != 0 {
        *builder = builder.executable();

        // This is a hack used to support `fs` accesses on macOS. We rewrites `fs` accesses to `gs` ones,
        // so executable pages must be also writable.
        *builder = builder.writable();
    }
}

const fn page_size() -> usize {
    if cfg!(target_arch = "x86_64") {
        0x1000
    } else if cfg!(target_arch = "aarch64") {
        0x4000
    } else {
        0
    }
}
