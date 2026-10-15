//! Fragmented MP4 output keeps sample tables bounded independently of video length.
use anyhow::{Context, Result, ensure};
use mp4::{Mp4Config, Mp4Writer, TrackConfig};
use std::io::{Cursor, Write};

fn atom(kind: &[u8; 4], payload: &[u8]) -> Result<Vec<u8>> {
    let length = u32::try_from(payload.len() + 8).context("MP4 box is too large")?;
    let mut out = Vec::with_capacity(payload.len() + 8);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    Ok(out)
}
fn children(bytes: &[u8]) -> Result<Vec<(&[u8; 4], &[u8])>> {
    let mut result = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        ensure!(bytes.len() - pos >= 8, "Truncated generated MP4 box");
        let size = u32::from_be_bytes(bytes[pos..pos + 4].try_into()?) as usize;
        ensure!(
            size >= 8 && size <= bytes.len() - pos,
            "Invalid generated MP4 box"
        );
        result.push((
            bytes[pos + 4..pos + 8].try_into()?,
            &bytes[pos + 8..pos + size],
        ));
        pos += size;
    }
    Ok(result)
}
fn duration_box(kind: &[u8; 4], input: &[u8], duration: u64) -> Result<Vec<u8>> {
    ensure!(
        input.len() >= 24 && input[0] == 0,
        "Unexpected MP4 duration header"
    );
    let mut out = vec![1, input[1], input[2], input[3]];
    out.extend_from_slice(&u64::from(u32::from_be_bytes(input[4..8].try_into()?)).to_be_bytes());
    out.extend_from_slice(&u64::from(u32::from_be_bytes(input[8..12].try_into()?)).to_be_bytes());
    let duration_offset = if kind == b"tkhd" { 20 } else { 16 };
    out.extend_from_slice(&input[12..duration_offset]);
    out.extend_from_slice(&duration.to_be_bytes());
    out.extend_from_slice(&input[duration_offset + 4..]);
    atom(kind, &out)
}
fn patch_track(bytes: &[u8], duration: u64) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for (kind, payload) in children(bytes)? {
        let value = if kind == b"tkhd" || kind == b"mdhd" {
            duration_box(kind, payload, duration)?
        } else if kind == b"mdia" {
            atom(kind, &patch_track(payload, duration)?)?
        } else {
            atom(kind, payload)?
        };
        out.extend_from_slice(&value);
    }
    Ok(out)
}
pub(crate) fn start(
    output: &mut impl Write,
    tracks: &[TrackConfig],
    durations: &[u64],
    timescale: u32,
) -> Result<()> {
    let mut encoded = Cursor::new(Vec::new());
    let mut writer = Mp4Writer::write_start(
        &mut encoded,
        &Mp4Config {
            major_brand: "isom".parse()?,
            minor_version: 512,
            compatible_brands: vec!["isom".parse()?, "iso6".parse()?, "avc1".parse()?],
            timescale,
        },
    )?;
    for track in tracks {
        writer.add_track(track)?;
    }
    writer.write_end()?;
    drop(writer);
    let duration = durations.iter().copied().max().unwrap_or(0);
    for (kind, payload) in children(encoded.get_ref())? {
        if kind == b"ftyp" {
            output.write_all(&atom(kind, payload)?)?;
        }
        if kind != b"moov" {
            continue;
        }
        let mut movie = Vec::new();
        let mut index = 0;
        for (child, data) in children(payload)? {
            let value = if child == b"mvhd" {
                duration_box(child, data, duration)?
            } else if child == b"trak" {
                let value = atom(child, &patch_track(data, durations[index])?)?;
                index += 1;
                value
            } else {
                atom(child, data)?
            };
            movie.extend_from_slice(&value);
        }
        let mut extend = Vec::new();
        let mut movie_duration = vec![1, 0, 0, 0];
        movie_duration.extend_from_slice(&duration.to_be_bytes());
        extend.extend_from_slice(&atom(b"mehd", &movie_duration)?);
        for index in 0..tracks.len() {
            let mut defaults = vec![0; 4];
            for field in [u32::try_from(index + 1)?, 1, 0, 0, 0] {
                defaults.extend_from_slice(&field.to_be_bytes());
            }
            extend.extend_from_slice(&atom(b"trex", &defaults)?);
        }
        movie.extend_from_slice(&atom(b"mvex", &extend)?);
        output.write_all(&atom(b"moov", &movie)?)?;
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)] // Explicit fields of one MP4 sample record.
pub(crate) fn sample(
    output: &mut impl Write,
    sequence: u32,
    track: u32,
    dts: u64,
    duration: u32,
    offset: i32,
    sync: bool,
    data: &[u8],
) -> Result<u64> {
    let mut mfhd = vec![0; 4];
    mfhd.extend_from_slice(&sequence.to_be_bytes());
    let mut tfhd = vec![0, 2, 0, 0];
    tfhd.extend_from_slice(&track.to_be_bytes());
    let mut tfdt = vec![1, 0, 0, 0];
    tfdt.extend_from_slice(&dts.to_be_bytes());
    let make_fragment = |data_offset: i32| -> Result<Vec<u8>> {
        let mut trun = vec![1, 0, 15, 1];
        trun.extend_from_slice(&1u32.to_be_bytes());
        trun.extend_from_slice(&data_offset.to_be_bytes());
        trun.extend_from_slice(&duration.to_be_bytes());
        trun.extend_from_slice(&u32::try_from(data.len())?.to_be_bytes());
        trun.extend_from_slice(&(if sync { 0x0200_0000u32 } else { 0x0101_0000u32 }).to_be_bytes());
        trun.extend_from_slice(&offset.to_be_bytes());
        let mut traf = atom(b"tfhd", &tfhd)?;
        traf.extend_from_slice(&atom(b"tfdt", &tfdt)?);
        traf.extend_from_slice(&atom(b"trun", &trun)?);
        let mut moof = atom(b"mfhd", &mfhd)?;
        moof.extend_from_slice(&atom(b"traf", &traf)?);
        atom(b"moof", &moof)
    };
    let size = make_fragment(0)?.len();
    output.write_all(&make_fragment(i32::try_from(size + 8)?)?)?;
    output.write_all(&u32::try_from(data.len() + 8)?.to_be_bytes())?;
    output.write_all(b"mdat")?;
    output.write_all(data)?;
    Ok((size + 8 + data.len()) as u64)
}
