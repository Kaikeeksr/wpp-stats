use memchr::memmem;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub fn duration_ms(path: &Path) -> Option<u32> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let mut f = BufReader::with_capacity(4096, File::open(path).ok()?);
    match ext.as_str() {
        "opus" | "ogg" => ogg_opus(&mut f),
        "mp4" | "mov" | "m4a" => mp4(&mut f),
        "mp3" => mp3(&mut f),
        _ => None,
    }
}

fn read_at<R: Read + Seek>(r: &mut R, pos: u64, buf: &mut [u8]) -> Option<()> {
    r.seek(SeekFrom::Start(pos)).ok()?;
    r.read_exact(buf).ok()
}

fn read_vec<R: Read + Seek>(r: &mut R, pos: u64, len: u64) -> Option<Vec<u8>> {
    let mut v = vec![0; len as usize];
    read_at(r, pos, &mut v)?;
    Some(v)
}

fn len_of<R: Seek>(r: &mut R) -> Option<u64> {
    r.seek(SeekFrom::End(0)).ok()
}

pub(crate) fn ogg_opus<R: Read + Seek>(r: &mut R) -> Option<u32> {
    let len = len_of(r)?;
    let head = read_vec(r, 0, len.min(4096))?;
    let p = memmem::find(&head, b"OpusHead")?;
    let pre_skip = i64::from(u16::from_le_bytes(
        head.get(p + 10..p + 12)?.try_into().ok()?,
    ));

    for tail_len in [8 << 10, 64 << 10] {
        let tail_len = len.min(tail_len);
        let tail = read_vec(r, len - tail_len, tail_len)?;
        if let Some(granule) = last_granule(&tail) {
            let ms = (granule - pre_skip).checked_mul(1000)? / 48_000;
            return u32::try_from(ms).ok();
        }
        if tail_len == len {
            break;
        }
    }
    None
}

fn last_granule(tail: &[u8]) -> Option<i64> {
    let mut end = tail.len();
    while let Some(p) = memmem::rfind(&tail[..end], b"OggS") {
        end = p;
        let Some(h) = tail.get(p..p + 27) else {
            continue;
        };
        let nseg = usize::from(h[26]);
        let Some(lacing) = tail.get(p + 27..p + 27 + nseg) else {
            continue;
        };
        let body: usize = lacing.iter().map(|&b| usize::from(b)).sum();
        if h[4] != 0 || p + 27 + nseg + body > tail.len() {
            continue;
        }
        let granule = i64::from_le_bytes(h[6..14].try_into().ok()?);
        if granule != -1 {
            return Some(granule);
        }
    }
    None
}

pub(crate) fn mp4<R: Read + Seek>(r: &mut R) -> Option<u32> {
    let len = len_of(r)?;
    let (moov, moov_end) = find_box(r, 0, len, b"moov")?;
    let (mvhd, _) = find_box(r, moov, moov_end, b"mvhd")?;
    let mut h = [0u8; 32];
    let v1 = {
        read_at(r, mvhd, &mut h[..1])?;
        h[0] == 1
    };
    let (timescale, duration) = if v1 {
        read_at(r, mvhd, &mut h[..32])?;
        (be32(&h[20..]), be64(&h[24..]))
    } else {
        read_at(r, mvhd, &mut h[..20])?;
        (be32(&h[12..]), u64::from(be32(&h[16..])))
    };
    if timescale == 0 || duration == u64::MAX || (!v1 && duration == u64::from(u32::MAX)) {
        return None;
    }
    u32::try_from(u128::from(duration) * 1000 / u128::from(timescale)).ok()
}

fn find_box<R: Read + Seek>(
    r: &mut R,
    mut pos: u64,
    end: u64,
    typ: &[u8; 4],
) -> Option<(u64, u64)> {
    while pos.checked_add(8)? <= end {
        let mut h = [0u8; 16];
        read_at(r, pos, &mut h[..8])?;
        let (hdr, size) = match be32(&h) {
            1 => {
                read_at(r, pos + 8, &mut h[8..16])?;
                (16, be64(&h[8..]))
            }
            0 => (8, end - pos),
            s => (8, u64::from(s)),
        };
        if size < hdr {
            return None;
        }
        let box_end = pos.checked_add(size)?;
        if &h[4..8] == typ {
            return Some((pos + hdr, box_end.min(end)));
        }
        if box_end > end {
            return None;
        }
        pos = box_end;
    }
    None
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes(b[..4].try_into().unwrap())
}

fn be64(b: &[u8]) -> u64 {
    u64::from_be_bytes(b[..8].try_into().unwrap())
}

pub(crate) fn mp3<R: Read + Seek>(r: &mut R) -> Option<u32> {
    let len = len_of(r)?;
    let mut id3 = [0u8; 10];
    read_at(r, 0, &mut id3)?;
    let start = if &id3[..3] == b"ID3" {
        let size = id3[6..10]
            .iter()
            .fold(0u64, |acc, &b| (acc << 7) | u64::from(b & 0x7F));
        10 + size + if id3[5] & 0x10 != 0 { 10 } else { 0 }
    } else {
        0
    };
    let buf = read_vec(r, start, (len.checked_sub(start)?).min(8192))?;
    let (off, f) = (0..buf.len().saturating_sub(4)).find_map(|i| Some((i, frame(&buf[i..])?)))?;

    let frame = &buf[off..];
    let side = match (f.mpeg1, f.mono) {
        (true, false) => 32,
        (true, true) | (false, false) => 17,
        (false, true) => 9,
    };
    let frames = match frame.get(4 + side..4 + side + 12) {
        Some(x) if &x[..4] == b"Xing" || &x[..4] == b"Info" => {
            (be32(&x[4..]) & 1 != 0).then(|| be32(&x[8..]))
        }
        _ => None,
    }
    .or_else(|| {
        let v = frame.get(36..36 + 18)?;
        (&v[..4] == b"VBRI").then(|| be32(&v[14..]))
    });
    let ms = match frames {
        Some(n) => u64::from(n) * u64::from(f.samples) * 1000 / u64::from(f.rate),
        None => (len - start - off as u64) * 8 / u64::from(f.kbps),
    };
    u32::try_from(ms).ok()
}

struct Frame {
    mpeg1: bool,
    mono: bool,
    kbps: u32,
    rate: u32,
    samples: u32,
}

fn frame(b: &[u8]) -> Option<Frame> {
    let h = be32(b.get(..4)?);
    if h >> 21 != 0x7FF {
        return None;
    }
    let version = (h >> 19) & 3;
    let layer = (h >> 17) & 3;
    let br = ((h >> 12) & 0xF) as usize;
    let sr = ((h >> 10) & 3) as usize;
    if version == 1 || layer == 0 || br == 0 || br == 15 || sr == 3 {
        return None;
    }
    let mpeg1 = version == 3;
    const BR_V1: [[u32; 15]; 3] = [
        [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
        ],
        [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
        ],
        [
            0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
        ],
    ];
    const BR_V2: [[u32; 15]; 2] = [
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
        [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
        ],
    ];
    let li = (layer - 1) as usize;
    let kbps = if mpeg1 {
        BR_V1[li][br]
    } else {
        BR_V2[usize::from(li == 2)][br]
    };
    let rate = match version {
        3 => [44_100, 48_000, 32_000][sr],
        2 => [22_050, 24_000, 16_000][sr],
        _ => [11_025, 12_000, 8_000][sr],
    };
    let samples = match (li, mpeg1) {
        (2, _) => 384,
        (1, _) | (0, true) => 1152,
        (0, false) => 576,
        _ => return None,
    };
    Some(Frame {
        mpeg1,
        mono: (h >> 6) & 3 == 3,
        kbps,
        rate,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn ogg_page(granule: i64, body_len: u8) -> Vec<u8> {
        let mut p = b"OggS".to_vec();
        p.push(0);
        p.push(0);
        p.extend(granule.to_le_bytes());
        p.extend([0; 12]);
        p.push(1);
        p.push(body_len);
        p.extend(vec![0xAB; usize::from(body_len)]);
        p
    }

    fn opus_file(last_granule: i64) -> Vec<u8> {
        let mut head = b"OpusHead\x01\x01".to_vec();
        head.extend(312u16.to_le_bytes());
        head.extend([0; 7]);
        let mut f = ogg_page(0, 0);
        f.extend(&head);
        f.extend(ogg_page(48_000 * 4 + 312, 50));
        f.extend(ogg_page(last_granule, 30));
        f
    }

    #[test]
    fn opus_last_granule() {
        let f = opus_file(48_000 * 8 + 24_000 + 312);
        assert_eq!(ogg_opus(&mut Cursor::new(f)), Some(8_500));
    }

    #[test]
    fn opus_granule_minus_one_uses_previous_page() {
        let f = opus_file(-1);
        assert_eq!(ogg_opus(&mut Cursor::new(f)), Some(4_000));
    }

    #[test]
    fn opus_last_page_beyond_small_tail() {
        let mut f = opus_file(-1);
        let mut page = ogg_page(48_000 * 9 + 312, 0);
        page.truncate(26);
        page.push(40);
        page.extend([255; 40]);
        page.extend(vec![0xAB; 40 * 255]);
        f.extend(page);
        assert_eq!(ogg_opus(&mut Cursor::new(f)), Some(9_000));
    }

    #[test]
    fn opus_garbage_is_none() {
        assert_eq!(ogg_opus(&mut Cursor::new(b"OggSxx".to_vec())), None);
        assert_eq!(ogg_opus(&mut Cursor::new(Vec::new())), None);
        let mut f = opus_file(48_000);
        f.truncate(f.len() - 10);
        assert_eq!(ogg_opus(&mut Cursor::new(f)), Some(4_000));
    }

    fn mp4_box(typ: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut b = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        b.extend(typ);
        b.extend(body);
        b
    }

    fn mvhd_v0(timescale: u32, duration: u32) -> Vec<u8> {
        let mut b = vec![0; 12];
        b.extend(timescale.to_be_bytes());
        b.extend(duration.to_be_bytes());
        b.extend([0; 80]);
        mp4_box(b"mvhd", &b)
    }

    #[test]
    fn mp4_moov_at_start_and_end() {
        let moov = mp4_box(b"moov", &mvhd_v0(1000, 90_500));
        let ftyp = mp4_box(b"ftyp", b"isom0000");
        let mdat = mp4_box(b"mdat", &[7; 5000]);
        let start = [ftyp.clone(), moov.clone(), mdat.clone()].concat();
        let end = [ftyp, mdat, moov].concat();
        assert_eq!(mp4(&mut Cursor::new(start)), Some(90_500));
        assert_eq!(mp4(&mut Cursor::new(end)), Some(90_500));
    }

    #[test]
    fn mp4_large_size_v1_and_size_zero() {
        let mut mdat = 1u32.to_be_bytes().to_vec();
        mdat.extend(b"mdat");
        mdat.extend(116u64.to_be_bytes());
        mdat.extend([0; 100]);
        let mut v1 = vec![1, 0, 0, 0];
        v1.extend([0; 16]);
        v1.extend(600u32.to_be_bytes());
        v1.extend(1_800u64.to_be_bytes());
        let mut moov = 0u32.to_be_bytes().to_vec();
        moov.extend(b"moov");
        moov.extend(mp4_box(b"mvhd", &v1));
        let f = [mdat, moov].concat();
        assert_eq!(mp4(&mut Cursor::new(f)), Some(3_000));
    }

    #[test]
    fn mp4_truncated_is_none() {
        let moov = mp4_box(b"moov", &mvhd_v0(1000, 5));
        let mut f = [mp4_box(b"ftyp", b"isom"), moov].concat();
        f.truncate(f.len() - 100);
        assert_eq!(mp4(&mut Cursor::new(f)), None);
        assert_eq!(mp4(&mut Cursor::new(vec![0, 0, 0, 3, b'a'])), None);
        let mut f = 1_000_000u32.to_be_bytes().to_vec();
        f.extend(b"mdat0000");
        assert_eq!(mp4(&mut Cursor::new(f)), None);
    }

    #[test]
    fn mp3_cbr_estimate() {
        let mut f = vec![0xFF, 0xFB, 0x90, 0x00];
        f.resize(16_000, 0);
        assert_eq!(mp3(&mut Cursor::new(f)), Some(1_000));
    }

    #[test]
    fn mp3_xing_frames() {
        let mut f = b"ID3\x04\x00\x00\x00\x00\x00\x05".to_vec();
        f.extend([0; 5]);
        f.extend([0xFF, 0xFB, 0x90, 0x00]);
        f.extend([0; 32]);
        f.extend(b"Xing");
        f.extend(1u32.to_be_bytes());
        f.extend(383u32.to_be_bytes());
        f.resize(2_000, 0);
        assert_eq!(mp3(&mut Cursor::new(f)), Some(10_004));
    }
}
