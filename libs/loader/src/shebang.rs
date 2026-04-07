//! MacTux shebang support.

use crate::Error;
use rtenv::rust::OwnedRtFd;
use std::io::{BufRead, BufReader};
use structures::error::LxError;

#[derive(Debug)]
pub struct Program {
    prog: Vec<u8>,
    arg: Option<Vec<u8>>,
    path: Vec<u8>,
}
impl Program {
    pub const MAGIC: &[u8] = b"#!";

    pub fn load(path: Vec<u8>) -> Result<Self, Error> {
        let fd = OwnedRtFd::open(path.clone()).map_err(Error::ReadImage)?;
        let mut buf_read = BufReader::new(fd);
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
            path,
        })
    }

    pub unsafe fn run(&self, args: &[&[u8]], envs: &[&[u8]]) -> ! {
        let mut argv = Vec::new();
        argv.push(&self.prog[..]);
        if let Some(opt) = &self.arg {
            argv.push(&opt[..]);
        }
        argv.push(&self.path);
        for i in args.iter().skip(1) {
            argv.push(*i);
        }

        unsafe {
            crate::Program::load(self.prog.clone())
                .unwrap_or_else(|e| {
                    eprintln!("mactux: failed to load script interpreter: {e}");
                    std::process::exit(1);
                })
                .run(&argv, envs);
        }
    }
}
