use std::{fmt::Display, str::FromStr};

/// Structure that represents to `/dev/fstab`.
#[derive(Debug, Clone)]
pub struct Fstab(pub Vec<FstabEntry>);
impl FromStr for Fstab {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let entries = s
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(str::trim)
            .map(FstabEntry::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Fstab(entries))
    }
}
impl Display for Fstab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in self.0.iter() {
            writeln!(f, "{i}")?;
        }
        Ok(())
    }
}

/// An entry in `/dev/fstab`.
#[derive(Debug, Clone)]
pub struct FstabEntry {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub options: String,
    pub dump: u32,
    pub pass: u32,
}
impl FromStr for FstabEntry {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        let device = parts.next().ok_or(ParseError)?.to_string();
        let mount_point = parts.next().ok_or(ParseError)?.to_string();
        let fs_type = parts.next().ok_or(ParseError)?.to_string();
        let options = parts.next().ok_or(ParseError)?.to_string();
        let dump = parts
            .next()
            .ok_or(ParseError)?
            .parse()
            .map_err(|_| ParseError)?;
        let pass = parts
            .next()
            .ok_or(ParseError)?
            .parse()
            .map_err(|_| ParseError)?;
        Ok(FstabEntry {
            device,
            mount_point,
            fs_type,
            options,
            dump,
            pass,
        })
    }
}
impl Display for FstabEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {}",
            self.device, self.mount_point, self.fs_type, self.options, self.dump, self.pass
        )
    }
}

/// An error while parsing a data structure.
#[derive(Debug, Clone)]
pub struct ParseError;
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse data structure")
    }
}
impl std::error::Error for ParseError {}
