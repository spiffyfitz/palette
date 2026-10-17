//! Bounded ISO BMFF readers. All offsets refer to validated slices; external
//! data references, unbounded recursion and unsupported item constructions fail.
use super::*;
use std::ops::Range;

#[derive(Clone)]
pub(super) struct BoxData<'a> {
    pub kind: [u8; 4],
    pub data: &'a [u8],
}
pub(super) fn boxes<'a>(
    mut data: &'a [u8],
    budget: &mut usize,
) -> Result<Vec<BoxData<'a>>, FileError> {
    let mut out = vec![];
    while !data.is_empty() {
        if *budget == 0 {
            return Err(invalid("Too many container boxes"));
        }
        *budget -= 1;
        let mut c = Cursor::new(data);
        let size = c.u32()?;
        let kind = c.array()?;
        let size = if size == 1 {
            usize::try_from(c.u64()?).map_err(|_| invalid("Box size overflow"))?
        } else if size == 0 {
            data.len()
        } else {
            size as usize
        };
        if size < c.position || size > data.len() {
            return Err(invalid("Box exceeds its container"));
        }
        out.push(BoxData {
            kind,
            data: &data[c.position..size],
        });
        data = &data[size..];
    }
    Ok(out)
}
pub(super) fn find<'a>(items: &[BoxData<'a>], kind: &[u8; 4]) -> Result<&'a [u8], FileError> {
    let mut matches = items.iter().filter(|b| &b.kind == kind);
    let item = matches
        .next()
        .ok_or_else(|| invalid(&format!("Missing {} box", String::from_utf8_lossy(kind))))?;
    if matches.next().is_some() {
        return Err(invalid("Duplicate structural box"));
    }
    Ok(item.data)
}
pub(super) struct Cursor<'a> {
    pub data: &'a [u8],
    pub position: usize,
}
impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], FileError> {
        let end = self
            .position
            .checked_add(n)
            .ok_or_else(|| invalid("Offset overflow"))?;
        let b = self
            .data
            .get(self.position..end)
            .ok_or_else(|| invalid("Truncated container data"))?;
        self.position = end;
        Ok(b)
    }
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], FileError> {
        Ok(self.take(N)?.try_into().expect("checked slice"))
    }
    pub fn u8(&mut self) -> Result<u8, FileError> {
        Ok(self.array::<1>()?[0])
    }
    pub fn u16(&mut self) -> Result<u16, FileError> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    pub fn u32(&mut self) -> Result<u32, FileError> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    pub fn u64(&mut self) -> Result<u64, FileError> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub fn full(&mut self, max: u8) -> Result<(u8, u32), FileError> {
        let word = self.u32()?;
        let version = (word >> 24) as u8;
        if version > max {
            return Err(invalid("Unsupported box version"));
        }
        Ok((version, word & 0xffffff))
    }
    pub fn id(&mut self, wide: bool) -> Result<u32, FileError> {
        if wide {
            self.u32()
        } else {
            Ok(u32::from(self.u16()?))
        }
    }
    fn number(&mut self, n: u8) -> Result<u64, FileError> {
        if !matches!(n, 0 | 4 | 8) {
            return Err(invalid("Unsupported item location field width"));
        }
        let mut number = 0;
        for b in self.take(n as usize)? {
            number = (number << 8) | u64::from(*b);
        }
        Ok(number)
    }
    pub fn done(&self) -> Result<(), FileError> {
        if self.position == self.data.len() {
            Ok(())
        } else {
            Err(invalid("Unexpected trailing box data"))
        }
    }
}

pub(super) struct Location {
    method: u16,
    extents: Vec<Range<usize>>,
}
pub(super) fn locations(data: &[u8]) -> Result<HashMap<u32, Location>, FileError> {
    let mut c = Cursor::new(data);
    let (version, flags) = c.full(2)?;
    if flags != 0 {
        return Err(invalid("Unsupported iloc flags"));
    }
    let a = c.u8()?;
    let b = c.u8()?;
    let (offset, length, base, index) =
        (a >> 4, a & 15, b >> 4, if version > 0 { b & 15 } else { 0 });
    let count = c.id(version == 2)?;
    if count > 1024 {
        return Err(invalid("Too many items"));
    }
    let mut locations = HashMap::new();
    for _ in 0..count {
        let id = c.id(version == 2)?;
        let method = if version > 0 { c.u16()? } else { 0 };
        if method > 1 || c.u16()? != 0 {
            return Err(invalid("External or unsupported item data construction"));
        }
        let base = c.number(base)?;
        let count = c.u16()?;
        if count == 0 || count > 8 {
            return Err(invalid("Unsupported item extent count"));
        }
        let mut extents = vec![];
        for _ in 0..count {
            if c.number(index)? != 0 {
                return Err(invalid("Indexed extents are not supported"));
            }
            let start = base
                .checked_add(c.number(offset)?)
                .ok_or_else(|| invalid("Extent overflow"))?;
            let end = start
                .checked_add(c.number(length)?)
                .ok_or_else(|| invalid("Extent overflow"))?;
            extents.push(
                usize::try_from(start).map_err(|_| invalid("Extent overflow"))?
                    ..usize::try_from(end).map_err(|_| invalid("Extent overflow"))?,
            );
        }
        if locations.insert(id, Location { method, extents }).is_some() {
            return Err(invalid("Duplicate item location"));
        }
    }
    c.done()?;
    Ok(locations)
}
impl Location {
    pub fn read(&self, file: &[u8], idat: &[u8]) -> Result<Vec<u8>, FileError> {
        let source = if self.method == 0 { file } else { idat };
        let mut out = vec![];
        for extent in &self.extents {
            if extent.end - extent.start > MAX_FILE
                || out.len() + (extent.end - extent.start) > MAX_FILE
            {
                return Err(invalid("Item exceeds input limit"));
            }
            out.extend_from_slice(
                source
                    .get(extent.clone())
                    .ok_or_else(|| invalid("Item extent is outside its data source"))?,
            );
        }
        Ok(out)
    }
}
