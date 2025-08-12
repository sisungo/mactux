#[macro_export]
macro_rules! posix_num {
    ($x:expr) => {
        match $x {
            -1 => Err(LxError::last_apple_error()),
            n => Ok(n as _),
        }
    };
}

#[macro_export]
macro_rules! posix_bi {
    ($x:expr) => {
        match $x {
            -1 => Err(LxError::last_apple_error()),
            _ => Ok(()),
        }
    };
}

pub fn c_path(mut dat: Vec<u8>) -> Vec<u8> {
    dat.push(0);
    dat
}
