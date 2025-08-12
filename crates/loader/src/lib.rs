mod auxv;
mod mmap;
mod stack;

use crate::auxv::AuxiliaryInfo;
use mmap::*;
use object::{
    LittleEndian, ReadCache,
    elf::{PT_INTERP, PT_LOAD, ProgramHeader64},
    read::elf::{ElfFile64, FileHeader, ProgramHeader},
};
use std::{
    io::{Read, Seek},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd},
};
use structures::fs::OpenFlags;

type ExecutableObject<'a> = ElfFile64<'a, LittleEndian, &'a ReadCache<IoFd<'a>>>;

/// A loaded Linux program.
pub struct Program {
    exec_fd: OwnedFd,

    interpreter: Option<Box<Program>>,
    phdr: *const u8,
    phent: usize,
    phnum: usize,
    entry: *const u8,

    base_map: MappedArea,
    _mapped_areas: Vec<MappedArea>,
}
impl Program {
    pub fn load<R: Into<OwnedFd>>(exec_fd: R) -> Result<Self, Error> {
        let exec_fd = exec_fd.into();
        let read_cache = ReadCache::new(IoFd(exec_fd.as_fd()));
        let main = ExecutableObject::parse(&read_cache).map_err(Error::Parse)?;
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
                    let mapped_area =
                        map_phdr(phdr, exec_fd.as_raw_fd(), base_map.addr()).map_err(Error::Map)?;
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

    pub unsafe fn run<'a>(
        &self,
        args: impl ExactSizeIterator<Item = &'a [u8]>,
        envs: impl Iterator<Item = &'a [u8]>,
    ) {
        let base = match &self.interpreter {
            Some(interp) => interp.base_map.addr() as usize,
            None => 0,
        };
        let entry = match &self.interpreter {
            Some(interp) => interp.entry,
            None => self.entry,
        };
        let auxv = AuxiliaryInfo {
            exec_fd: self.exec_fd.as_raw_fd() as _,
            phdr_base: self.phdr as usize,
            phdr_size: self.phent as usize,
            phdr_count: self.phnum as usize,
            entry: self.entry as usize,
            base,
        };
        stack::jump(entry, args, envs, auxv);
    }
}

#[derive(Debug)]
struct IoFd<'a>(BorrowedFd<'a>);
impl Read for IoFd<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            match libc::read(self.0.as_raw_fd(), buf.as_mut_ptr() as _, buf.len()) {
                -1 => Err(std::io::Error::last_os_error()),
                n => Ok(n as usize),
            }
        }
    }
}
impl Seek for IoFd<'_> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let offset = match pos {
            std::io::SeekFrom::Start(o) => o as _,
            std::io::SeekFrom::End(o) => o as _,
            std::io::SeekFrom::Current(o) => o as _,
        };
        let whence = match pos {
            std::io::SeekFrom::Start(_) => libc::SEEK_SET,
            std::io::SeekFrom::End(_) => libc::SEEK_END,
            std::io::SeekFrom::Current(_) => libc::SEEK_CUR,
        };
        unsafe {
            match libc::lseek(self.0.as_raw_fd(), offset, whence) {
                -1 => Err(std::io::Error::last_os_error()),
                n => Ok(n as u64),
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Read(std::io::Error),
    Parse(object::Error),
    Map(std::io::Error),
    NoLoadHeader,
    IncompatibleElf,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(e) => write!(f, "Read error: {}", e),
            Self::Parse(e) => write!(f, "Parse error: {}", e),
            Self::Map(e) => write!(f, "Mapping error: {}", e),
            Self::NoLoadHeader => write!(f, "No load header in the program"),
            Self::IncompatibleElf => write!(f, "Incompatible ELF format"),
        }
    }
}
impl std::error::Error for Error {}

fn map_base(main: &ExecutableObject) -> Result<MappedArea, Error> {
    if main.elf_header().e_type.get(LittleEndian) == object::elf::ET_DYN {
        let top_phdr = main
            .elf_program_headers()
            .iter()
            .filter(|x| x.p_type(LittleEndian) == PT_LOAD)
            .max_by_key(|x| x.p_vaddr(LittleEndian))
            .ok_or_else(|| Error::NoLoadHeader)?;
        let max_addr = top_phdr.p_vaddr(LittleEndian) + top_phdr.p_memsz(LittleEndian);
        unsafe {
            MappedArea::builder()
                .len(max_addr as _)
                .build()
                .map_err(Error::Map)
        }
    } else {
        Ok(MappedArea::null())
    }
}

fn read_interp(
    phdr: &ProgramHeader64<LittleEndian>,
    read_cache: &ReadCache<IoFd>,
) -> Result<OwnedFd, Error> {
    let path = phdr
        .interpreter(LittleEndian, read_cache)
        .map_err(Error::Parse)?
        .ok_or(Error::IncompatibleElf)?;
    let interp_fd = rtenv::fs::open(path.into(), OpenFlags::O_CLOEXEC | OpenFlags::O_RDONLY, 0)
        .map_err(|_| Error::IncompatibleElf)?;
    if rtenv::vfd::get(interp_fd).is_some() {
        return Err(Error::IncompatibleElf);
    }
    unsafe { Ok(OwnedFd::from_raw_fd(interp_fd)) }
}

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
