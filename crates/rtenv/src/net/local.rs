use libc::c_char;
use structures::{error::LxError, net::SockAddrUn};

pub fn apple_sockaddr(linux: SockAddrUn, len: usize, create: bool) -> Result<libc::sockaddr_un, LxError> {
    let path = if linux.sun_path[0] == 0 {
        linux.sun_path[..len].iter().map(|x| *x as u8).collect()
    } else {
        let zero_offset = *linux.sun_path.iter().find(|x| **x == 0).ok_or(LxError::EINVAL)? as usize;
        linux.sun_path[..zero_offset].iter().map(|x| *x as u8).collect()
    };
    let path = crate::fs::get_sock_path(path, create)?.iter().map(|x| *x as c_char).collect::<Vec<i8>>();
    let mut apple_path = [0; _];
    if path.len() > size_of_val(&apple_path) {
        return Err(LxError::ENOMEM);
    }
    apple_path[..path.len()].copy_from_slice(&path);
    apple_path[path.len()] = 0;

    Ok(libc::sockaddr_un {
        sun_len: size_of::<libc::sockaddr_un>() as _,
        sun_family: libc::AF_LOCAL as _,
        sun_path: apple_path,
    })
}