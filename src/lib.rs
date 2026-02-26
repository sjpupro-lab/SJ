use anyhow::{bail, Result};
use std::collections::HashMap;

pub const W: u32 = 512;
pub const H: u32 = 512;
pub const PIXELS: usize = (W as usize) * (H as usize);
pub const MAGIC: &[u8; 4] = b"CVP1";
pub const RG_LIMIT: u64 = (1u64 << 63) / 16; // == floor(2^64 / 32) but safe in u64 ops
// NOTE: 2^64 doesn't fit u64, so use equivalent: floor(2^64/32) == 2^59
// 2^59 == (1<<63)/16
pub const RG_LIMIT_EXACT: u64 = 1u64 << 59;

#[inline]
pub fn lane_k(step: u32) -> (u8, u64) {
    // 0=R, 1=G
    if (step & 1) == 1 {
        (0, ((step + 1) >> 1) as u64)
    } else {
        (1, (step >> 1) as u64)
    }
}

#[inline]
pub fn pidx_of(x: u32, y: u32) -> u32 {
    (y << 9) + x
}

#[derive(Clone, Debug)]
pub struct RGCanvas {
    pub r: Vec<u64>,
    pub g: Vec<u64>,
}

impl RGCanvas {
    pub fn new(full: bool) -> Self {
        let init = if full { RG_LIMIT_EXACT } else { 0 };
        Self {
            r: vec![init; PIXELS],
            g: vec![init; PIXELS],
        }
    }
}

#[derive(Clone, Debug)]
pub struct ABitset {
    // pidx -> pages: page -> mask
    pub db: HashMap<u32, HashMap<u32, u64>>,
}

impl ABitset {
    pub fn new() -> Self {
        Self { db: HashMap::new() }
    }

    #[inline]
    pub fn set_step(&mut self, pidx: u32, step: u32) {
        let page = step >> 6;
        let bit = step & 63;
        let m = 1u64 << bit;
        let pages = self.db.entry(pidx).or_insert_with(HashMap::new);
        let entry = pages.entry(page).or_insert(0);
        *entry |= m;
    }

    #[inline]
    pub fn clear_step(&mut self, pidx: u32, step: u32) -> Result<()> {
        let page = step >> 6;
        let bit = step & 63;
        let m = 1u64 << bit;

        let pages = self.db.get_mut(&pidx).ok_or_else(|| anyhow::anyhow!("A mismatch: missing pidx"))?;
        let word = pages.get_mut(&page).ok_or_else(|| anyhow::anyhow!("A mismatch: missing page"))?;
        if ((*word >> bit) & 1) == 0 {
            bail!("A mismatch: bit already 0");
        }
        *word &= !m;

        if *word == 0 {
            pages.remove(&page);
            if pages.is_empty() {
                self.db.remove(&pidx);
            }
        }
        Ok(())
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }
}

/// Build step->pidx index by scanning A exactly once.
/// No step_to_x stored in RAW. Cache is in-memory only.
pub fn build_step_index_from_a(a: &ABitset, n: u32) -> Result<Vec<u32>> {
    let mut step_to_pidx = vec![0u32; (n as usize) + 1];
    let mut seen = vec![0u8; (n as usize) + 1];

    for (&pidx, pages) in a.db.iter() {
        for (&page, &mask) in pages.iter() {
            let base = page << 6;
            let mut m = mask;
            while m != 0 {
                let tz = m.trailing_zeros() as u32;
                let step = base + tz;
                if step >= 1 && step <= n {
                    let idx = step as usize;
                    if seen[idx] != 0 {
                        bail!("step collision in A: step={}", step);
                    }
                    seen[idx] = 1;
                    step_to_pidx[idx] = pidx;
                }
                m &= m - 1;
            }
        }
    }

    for s in 1..=n as usize {
        if seen[s] == 0 {
            bail!("missing step in A: step={}", s);
        }
    }

    Ok(step_to_pidx)
}

/// CVP1 RAW packing
pub fn raw_pack(rg: &RGCanvas, a: &ABitset, n: u32) -> Vec<u8> {
    let mut out = Vec::new();

    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&W.to_le_bytes());
    out.extend_from_slice(&H.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&RG_LIMIT_EXACT.to_le_bytes());

    // planes
    for &v in rg.r.iter() { out.extend_from_slice(&v.to_le_bytes()); }
    for &v in rg.g.iter() { out.extend_from_slice(&v.to_le_bytes()); }

    // A entry count
    let entry_count = a.db.len() as u32;
    out.extend_from_slice(&entry_count.to_le_bytes());

    // entries
    for (&pidx, pages) in a.db.iter() {
        out.extend_from_slice(&pidx.to_le_bytes());
        let pcnt = pages.len() as u16;
        out.extend_from_slice(&pcnt.to_le_bytes());
        for (&page, &mask) in pages.iter() {
            out.extend_from_slice(&page.to_le_bytes());
            out.extend_from_slice(&mask.to_le_bytes());
        }
    }

    out
}

pub fn raw_unpack(raw: &[u8]) -> Result<(RGCanvas, ABitset, u32)> {
    let mut off = 0usize;

    if raw.len() < 4 { bail!("raw too small"); }
    if &raw[0..4] != MAGIC { bail!("bad magic"); }
    off += 4;

    let read_u32 = |buf: &[u8], off: &mut usize| -> Result<u32> {
        if *off + 4 > buf.len() { bail!("unexpected eof"); }
        let v = u32::from_le_bytes(buf[*off..*off+4].try_into().unwrap());
        *off += 4;
        Ok(v)
    };
    let read_u16 = |buf: &[u8], off: &mut usize| -> Result<u16> {
        if *off + 2 > buf.len() { bail!("unexpected eof"); }
        let v = u16::from_le_bytes(buf[*off..*off+2].try_into().unwrap());
        *off += 2;
        Ok(v)
    };
    let read_u64 = |buf: &[u8], off: &mut usize| -> Result<u64> {
        if *off + 8 > buf.len() { bail!("unexpected eof"); }
        let v = u64::from_le_bytes(buf[*off..*off+8].try_into().unwrap());
        *off += 8;
        Ok(v)
    };

    let w = read_u32(raw, &mut off)?;
    let h = read_u32(raw, &mut off)?;
    let n = read_u32(raw, &mut off)?;
    let rg_limit = read_u64(raw, &mut off)?;

    if w != W || h != H { bail!("bad dims: {}x{}", w, h); }
    if rg_limit != RG_LIMIT_EXACT { bail!("RG_LIMIT mismatch"); }

    let mut rg = RGCanvas::new(false);
    for i in 0..PIXELS { rg.r[i] = read_u64(raw, &mut off)?; }
    for i in 0..PIXELS { rg.g[i] = read_u64(raw, &mut off)?; }

    let entry_count = read_u32(raw, &mut off)?;
    let mut a = ABitset::new();

    for _ in 0..entry_count {
        let pidx = read_u32(raw, &mut off)?;
        let pcnt = read_u16(raw, &mut off)? as u32;
        let mut pages: HashMap<u32, u64> = HashMap::new();
        for _ in 0..pcnt {
            let page = read_u32(raw, &mut off)?;
            let mask = read_u64(raw, &mut off)?;
            pages.insert(page, mask);
        }
        a.db.insert(pidx, pages);
    }

    Ok((rg, a, n))
}

/// Encode (Erase): start FULL, step N..1, A set then RG -= k
pub fn encode_erase(payload: &[u8]) -> Result<Vec<u8>> {
    let n = payload.len() as u32;
    if n == 0 { bail!("empty payload"); }

    let mut rg = RGCanvas::new(true);
    let mut a = ABitset::new();

    for step in (1..=n).rev() {
        let x = payload[(step - 1) as usize] as u32;
        let y = step & 511;
        let pidx = pidx_of(x, y) as usize;

        a.set_step(pidx as u32, step); // A first

        let (lane, k) = lane_k(step);
        if lane == 0 {
            let v = rg.r[pidx];
            if v < k { bail!("R underflow encode"); }
            rg.r[pidx] = v - k;
        } else {
            let v = rg.g[pidx];
            if v < k { bail!("G underflow encode"); }
            rg.g[pidx] = v - k;
        }
    }

    Ok(raw_pack(&rg, &a, n))
}

/// Decode (Fill): build step->pidx cache by 1 scan of A, then step N..1:
/// A clear then RG += k, verify A empty and RG FULL.
pub fn decode_fill(raw: &[u8]) -> Result<Vec<u8>> {
    let (mut rg, mut a, n) = raw_unpack(raw)?;
    let step_to_pidx = build_step_index_from_a(&a, n)?;

    let mut out = vec![0u8; n as usize];

    for step in (1..=n).rev() {
        let pidx_u32 = step_to_pidx[step as usize];
        let pidx = pidx_u32 as usize;

        let x = (pidx_u32 & 511) as u8;
        out[(step - 1) as usize] = x;

        a.clear_step(pidx_u32, step)?; // A first

        let (lane, k) = lane_k(step);
        if lane == 0 {
            let v = rg.r[pidx] + k;
            if v > RG_LIMIT_EXACT { bail!("R overflow decode"); }
            rg.r[pidx] = v;
        } else {
            let v = rg.g[pidx] + k;
            if v > RG_LIMIT_EXACT { bail!("G overflow decode"); }
            rg.g[pidx] = v;
        }
    }

    if !a.is_empty() { bail!("A not empty after decode"); }
    if rg.r.iter().any(|&v| v != RG_LIMIT_EXACT) || rg.g.iter().any(|&v| v != RG_LIMIT_EXACT) {
        bail!("RG not FULL after decode");
    }

    Ok(out)
}