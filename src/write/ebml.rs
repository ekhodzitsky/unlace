use super::WriteError;

pub(super) const UNKNOWN: [u8; 8] = [1, 255, 255, 255, 255, 255, 255, 255];
pub(super) const MAX_SIZE: u64 = (1 << 56) - 2;

pub(super) fn size(n: u64, width: Option<usize>) -> Result<([u8; 8], usize), WriteError> {
    let w = match width {
        Some(w) => w,
        None => (1..=8)
            .find(|w| n < (1u64 << (7 * w)) - 1)
            .ok_or(WriteError::Limit("element size"))?,
    };
    if !(1..=8).contains(&w) || n >= (1u64 << (7 * w)) - 1 {
        return Err(WriteError::Limit("element size"));
    }
    Ok(((n | (1u64 << (7 * w))).to_be_bytes(), w))
}
pub(super) fn vint(n: u64) -> Result<([u8; 8], usize), WriteError> {
    let w = (1..=8)
        .find(|w| n < (1u64 << (7 * w)))
        .ok_or(WriteError::Invalid("VINT range"))?;
    Ok(((n | (1u64 << (7 * w))).to_be_bytes(), w))
}

pub(super) fn header(id: u32, n: u64) -> Result<([u8; 12], usize), WriteError> {
    let id_bytes = id.to_be_bytes();
    let iw = ((32 - id.leading_zeros()) as usize).div_ceil(8);
    let (size_bytes, sw) = size(n, None)?;
    let mut bytes = [0; 12];
    bytes[..iw].copy_from_slice(&id_bytes[4 - iw..]);
    let width = iw + sw;
    bytes[iw..width].copy_from_slice(&size_bytes[8 - sw..]);
    Ok((bytes, width))
}
pub(super) fn wire_size(n: u64) -> Result<u64, WriteError> {
    let (_, w) = size(n, None)?;
    Ok(4 + w as u64 + n)
}

pub(super) struct Bytes {
    pub data: Vec<u8>,
    pub limit: usize,
}
impl Bytes {
    pub fn new(limit: usize) -> Self {
        Self {
            data: Vec::new(),
            limit,
        }
    }
    pub fn clear(&mut self) {
        self.data.clear();
    }
    pub fn raw(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        if bytes.len() > self.limit.saturating_sub(self.data.len()) {
            return Err(WriteError::Limit("serialization buffer"));
        }
        self.data
            .try_reserve(bytes.len())
            .map_err(|_| WriteError::Limit("allocation"))?;
        self.data.extend_from_slice(bytes);
        Ok(())
    }
    pub fn id(&mut self, id: u32) -> Result<(), WriteError> {
        let bytes = id.to_be_bytes();
        let w = ((32 - id.leading_zeros()) as usize).div_ceil(8);
        self.raw(&bytes[4 - w..])
    }
    pub fn header(&mut self, id: u32, n: u64) -> Result<(), WriteError> {
        self.id(id)?;
        let (bytes, w) = size(n, None)?;
        self.raw(&bytes[8 - w..])
    }
    pub fn prepend_header(&mut self, id: u32, n: u64) -> Result<(), WriteError> {
        let (header, width) = header(id, n)?;
        let old = self.data.len();
        self.raw(&header[..width])?;
        self.data.copy_within(0..old, width);
        self.data[..width].copy_from_slice(&header[..width]);
        Ok(())
    }
    pub fn leaf(&mut self, id: u32, bytes: &[u8]) -> Result<(), WriteError> {
        self.header(id, bytes.len() as u64)?;
        self.raw(bytes)
    }
    pub fn uint(&mut self, id: u32, n: u64) -> Result<(), WriteError> {
        let w = ((64 - n.leading_zeros()) as usize).div_ceil(8).max(1);
        self.leaf(id, &n.to_be_bytes()[8 - w..])
    }
    pub fn signed(&mut self, id: u32, n: i64) -> Result<(), WriteError> {
        self.leaf(id, &n.to_be_bytes())
    }
    pub fn master(
        &mut self,
        id: u32,
        f: impl FnOnce(&mut Self) -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        self.id(id)?;
        let at = self.data.len();
        self.raw(&UNKNOWN)?;
        f(self)?;
        let length = self.data.len() - at - 8;
        let (bytes, _) = size(length as u64, Some(8))?;
        let target = self
            .data
            .get_mut(at..at + 8)
            .ok_or(WriteError::Invalid("master bounds"))?;
        target.copy_from_slice(&bytes);
        Ok(())
    }
}
