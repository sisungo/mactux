use crate::{Error, IoFd};
use std::{
    io::{BufRead, BufReader, Seek, SeekFrom},
    os::fd::{AsFd, FromRawFd, OwnedFd},
};
use structures::{
    error::LxError,
    fs::{FileMode, OpenFlags},
};

#[derive(Debug)]
pub struct Program {
    prog: Vec<u8>,
    arg: Option<Vec<u8>>,
}
impl Program {
    pub const MAGIC: &[u8] = b"#!";

    pub fn load(exec_fd: OwnedFd) -> Result<Self, Error> {
        let mut io_fd = IoFd(exec_fd.as_fd());
        io_fd
            .seek(SeekFrom::Start(0))
            .map_err(|x| Error::ReadImage(x.into()))?;
        let mut buf_read = BufReader::new(io_fd);
        let mut first_line = Vec::with_capacity(32);
        buf_read
            .read_until(b'\n', &mut first_line)
            .map_err(|x| Error::ReadImage(x.into()))?;
        if first_line.len() <= 3 {
            return Err(Error::ImageFormat(String::from("invalid shebang line")));
        }
        let interp = first_line[2..].trim_ascii();
        let (prog, arg) = match interp.split_once(|x| *x == b' ') {
            Some((a, b)) => (a, Some(b)),
            None => (interp, None),
        };
        if !prog.starts_with(b"/") {
            return Err(Error::ReadImage(LxError::ENOENT));
        }
        Ok(Self {
            prog: prog.into(),
            arg: arg.map(|x| x.into()),
        })
    }

    pub unsafe fn run<'a, 'b>(&self, args: &[&[u8]], envs: &[&[u8]]) {
        let mut argv = Vec::new();
        argv.push(&self.prog[..]);
        if let Some(opt) = &self.arg {
            argv.push(&opt[..]);
        }
        for i in args {
            argv.push(*i);
        }

        unsafe {
            let interp_fd = rtenv::fs::open(
                self.prog.clone(),
                OpenFlags::O_CLOEXEC | OpenFlags::O_RDONLY,
                FileMode(0),
            );
            let interp_fd = match interp_fd {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("mactux: failed to open script interpreter: {e}");
                    std::process::exit(1);
                }
            };
            if rtenv::vfd::get(interp_fd).is_some() {
                eprintln!("mactux: failed to load script interpreter: is virtual file descriptor");
                std::process::exit(1);
            }
            crate::Program::load(OwnedFd::from_raw_fd(interp_fd))
                .unwrap_or_else(|e| {
                    eprintln!("mactux: failed to load script interpreter: {e}");
                    std::process::exit(1);
                })
                .run(&argv, envs);
        }
    }
}
