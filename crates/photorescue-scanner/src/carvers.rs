use crc32fast::Hasher;
use photorescue_domain::{CandidateStatus, ImageFormat};
use std::io::{Read, Seek, SeekFrom};

const CARVE_BUFFER_SIZE: usize = 1024 * 1024;
const MIN_PARTIAL_IMAGE_BYTES: u64 = 16;
const SIGNATURE_TAIL: usize = 32;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CarveResult {
    pub length: u64,
    pub confidence: u8,
    pub status: CandidateStatus,
}

impl CarveResult {
    fn complete(length: u64, confidence: u8) -> Self {
        Self {
            length,
            confidence,
            status: CandidateStatus::Found,
        }
    }

    fn partial(length: u64, confidence: u8) -> Self {
        Self {
            length,
            confidence,
            status: CandidateStatus::Partial,
        }
    }

    fn corrupted(length: u64, confidence: u8) -> Self {
        Self {
            length,
            confidence,
            status: CandidateStatus::Corrupted,
        }
    }
}

pub(crate) fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(ImageFormat::Png);
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some(ImageFormat::Jpeg);
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(ImageFormat::Webp);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(ImageFormat::Gif);
    }
    if bytes.starts_with(b"BM") {
        return Some(ImageFormat::Bmp);
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && is_heic_brand(&bytes[8..12]) {
        return Some(ImageFormat::Heic);
    }
    None
}

pub(crate) fn carve_at<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
    format: ImageFormat,
    max_file_size: u64,
    allow_partial: bool,
) -> Option<CarveResult> {
    match format {
        ImageFormat::Jpeg => carve_jpeg(reader, offset, max_file_size, allow_partial),
        ImageFormat::Png => carve_png(reader, offset, max_file_size, allow_partial),
        ImageFormat::Webp => carve_webp(reader, offset, max_file_size),
        ImageFormat::Heic => carve_heic(reader, offset, max_file_size),
        ImageFormat::Bmp => carve_bmp(reader, offset, max_file_size),
        ImageFormat::Gif => carve_gif(reader, offset, max_file_size),
    }
}

fn read_at<R: Read + Seek>(reader: &mut R, offset: u64, length: usize) -> Option<Vec<u8>> {
    let mut data = vec![0_u8; length];
    reader.seek(SeekFrom::Start(offset)).ok()?;
    reader.read_exact(&mut data).ok()?;
    Some(data)
}

fn read_u8_at<R: Read + Seek>(reader: &mut R, offset: u64) -> Option<u8> {
    read_at(reader, offset, 1).map(|bytes| bytes[0])
}

fn carve_jpeg<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    max_file_size: u64,
    allow_partial: bool,
) -> Option<CarveResult> {
    if read_at(reader, start, 3)?.as_slice() != b"\xff\xd8\xff" {
        return None;
    }

    let limit = start.checked_add(max_file_size)?;
    let mut position = start + 2;
    let mut saw_segment = false;
    let mut saw_sos = false;
    let mut last_structured_position = start + 3;

    while position + 1 < limit {
        let Some(byte) = read_u8_at(reader, position) else {
            return jpeg_unfinished_result(
                reader,
                start,
                position,
                limit,
                allow_partial,
                saw_sos,
                saw_segment,
                last_structured_position,
            );
        };
        if byte != 0xFF {
            position += 1;
            continue;
        }

        while position < limit && read_u8_at(reader, position)? == 0xFF {
            position += 1;
        }
        let Some(marker) = read_u8_at(reader, position) else {
            return jpeg_unfinished_result(
                reader,
                start,
                position,
                limit,
                allow_partial,
                saw_sos,
                saw_segment,
                last_structured_position,
            );
        };
        position += 1;

        match marker {
            0xD9 => {
                let length = position - start;
                return (length >= 4).then_some(CarveResult::complete(length, 65));
            }
            0xD8 | 0x01 | 0xD0..=0xD7 => continue,
            0xDA => {
                saw_sos = true;
                let Some(length_bytes) = read_at(reader, position, 2) else {
                    return jpeg_unfinished_result(
                        reader,
                        start,
                        position,
                        limit,
                        allow_partial,
                        saw_sos,
                        saw_segment,
                        last_structured_position,
                    );
                };
                let segment_length = u16::from_be_bytes([length_bytes[0], length_bytes[1]]) as u64;
                if segment_length < 2 || position + segment_length > limit {
                    return jpeg_damaged_result(
                        reader,
                        start,
                        position,
                        limit,
                        allow_partial,
                        saw_segment,
                        last_structured_position,
                    );
                }
                position += segment_length;
                last_structured_position = position;
                if let Some(end) = scan_jpeg_entropy(reader, position, limit) {
                    return Some(CarveResult::complete(end - start, 98));
                }
                return jpeg_unfinished_result(
                    reader,
                    start,
                    position,
                    limit,
                    allow_partial,
                    saw_sos,
                    saw_segment,
                    last_structured_position,
                );
            }
            _ => {
                let Some(length_bytes) = read_at(reader, position, 2) else {
                    return jpeg_unfinished_result(
                        reader,
                        start,
                        position,
                        limit,
                        allow_partial,
                        saw_sos,
                        saw_segment,
                        last_structured_position,
                    );
                };
                let segment_length = u16::from_be_bytes([length_bytes[0], length_bytes[1]]) as u64;
                if segment_length < 2 || position + segment_length > limit {
                    return jpeg_damaged_result(
                        reader,
                        start,
                        position,
                        limit,
                        allow_partial,
                        saw_segment,
                        last_structured_position,
                    );
                }
                saw_segment = true;
                position += segment_length;
                last_structured_position = position;
            }
        }
    }
    jpeg_unfinished_result(
        reader,
        start,
        position,
        limit,
        allow_partial,
        saw_sos,
        saw_segment,
        last_structured_position,
    )
}

fn scan_jpeg_entropy<R: Read + Seek>(reader: &mut R, start: u64, limit: u64) -> Option<u64> {
    reader.seek(SeekFrom::Start(start)).ok()?;
    let mut buffer = vec![0_u8; CARVE_BUFFER_SIZE];
    let mut position = start;
    let mut previous_was_ff = false;

    while position < limit {
        let wanted = (limit - position).min(buffer.len() as u64) as usize;
        let bytes_read = reader.read(&mut buffer[..wanted]).ok()?;
        if bytes_read == 0 {
            return None;
        }

        for (index, byte) in buffer[..bytes_read].iter().copied().enumerate() {
            if previous_was_ff && byte == 0xD9 {
                return Some(position + index as u64 + 1);
            }
            previous_was_ff = byte == 0xFF;
        }
        position += bytes_read as u64;
    }
    None
}

fn jpeg_unfinished_result<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    position: u64,
    limit: u64,
    allow_partial: bool,
    saw_sos: bool,
    saw_segment: bool,
    last_structured_position: u64,
) -> Option<CarveResult> {
    if !allow_partial {
        return None;
    }

    if saw_sos {
        let end = partial_end_before_next_signature(reader, position, limit).unwrap_or(limit);
        let length = end.saturating_sub(start);
        return (length >= MIN_PARTIAL_IMAGE_BYTES).then_some(CarveResult::partial(length, 68));
    }

    if saw_segment {
        let end = last_structured_position.max(start + 3).min(limit);
        let length = end.saturating_sub(start);
        return (length >= MIN_PARTIAL_IMAGE_BYTES).then_some(CarveResult::corrupted(length, 42));
    }

    None
}

fn jpeg_damaged_result<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    position: u64,
    limit: u64,
    allow_partial: bool,
    saw_segment: bool,
    last_structured_position: u64,
) -> Option<CarveResult> {
    if !allow_partial || !saw_segment {
        return None;
    }

    let fallback = last_structured_position.max(position).min(limit);
    let end = partial_end_before_next_signature(reader, fallback, limit).unwrap_or(fallback);
    let length = end.saturating_sub(start);
    (length >= MIN_PARTIAL_IMAGE_BYTES).then_some(CarveResult::corrupted(length, 45))
}

fn partial_end_before_next_signature<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    limit: u64,
) -> Option<u64> {
    reader.seek(SeekFrom::Start(start)).ok()?;
    let mut buffer = vec![0_u8; CARVE_BUFFER_SIZE];
    let mut tail = Vec::new();
    let mut position = start;

    while position < limit {
        let wanted = (limit - position).min(CARVE_BUFFER_SIZE as u64) as usize;
        let bytes_read = reader.read(&mut buffer[..wanted]).ok()?;
        if bytes_read == 0 {
            break;
        }

        let base = position.saturating_sub(tail.len() as u64);
        let mut window = Vec::with_capacity(tail.len() + bytes_read);
        window.extend_from_slice(&tail);
        window.extend_from_slice(&buffer[..bytes_read]);

        for index in 0..window.len() {
            let absolute = base + index as u64;
            if absolute >= start && detect_format(&window[index..]).is_some() {
                return Some(absolute);
            }
        }

        let keep = SIGNATURE_TAIL.min(window.len());
        tail.clear();
        tail.extend_from_slice(&window[window.len() - keep..]);
        position += bytes_read as u64;
    }

    (position > start).then_some(position)
}

fn carve_png<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    max_file_size: u64,
    allow_partial: bool,
) -> Option<CarveResult> {
    if read_at(reader, start, 8)?.as_slice() != b"\x89PNG\r\n\x1a\n" {
        return None;
    }

    let mut position = start + 8;
    let limit = start.checked_add(max_file_size)?;
    let mut integrity_ok = true;
    let mut saw_ihdr = false;
    let mut saw_idat = false;
    let mut last_good_position = position;

    loop {
        if position + 12 > limit {
            return png_unfinished_result(
                allow_partial,
                start,
                last_good_position,
                saw_ihdr,
                saw_idat,
                integrity_ok,
            );
        }
        let Some(header) = read_at(reader, position, 8) else {
            return png_unfinished_result(
                allow_partial,
                start,
                last_good_position,
                saw_ihdr,
                saw_idat,
                integrity_ok,
            );
        };
        let data_length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as u64;
        let chunk_type = &header[4..8];
        if !chunk_type.iter().all(u8::is_ascii_alphabetic) {
            return png_damaged_result(
                allow_partial,
                start,
                last_good_position,
                saw_ihdr,
                saw_idat,
            );
        }
        if data_length > max_file_size || position + 12 + data_length > limit {
            return png_unfinished_result(
                allow_partial,
                start,
                last_good_position,
                saw_ihdr,
                saw_idat,
                integrity_ok,
            );
        }

        let Some(crc_ok) = verify_png_crc(reader, position + 8, chunk_type, data_length) else {
            return png_unfinished_result(
                allow_partial,
                start,
                last_good_position,
                saw_ihdr,
                saw_idat,
                integrity_ok,
            );
        };
        integrity_ok &= crc_ok;

        if chunk_type == b"IHDR" {
            if saw_ihdr || data_length != 13 || position != start + 8 {
                return png_damaged_result(
                    allow_partial,
                    start,
                    last_good_position,
                    true,
                    saw_idat,
                );
            }
            saw_ihdr = true;
        } else if !saw_ihdr {
            return None;
        } else if chunk_type == b"IDAT" {
            saw_idat = true;
        }
        position += 12 + data_length;
        last_good_position = position;
        if chunk_type == b"IEND" {
            if !saw_ihdr || data_length != 0 || !saw_idat {
                return Some(CarveResult::corrupted(position - start, 45));
            }
            return Some(if integrity_ok {
                CarveResult::complete(position - start, 100)
            } else {
                CarveResult::corrupted(position - start, 70)
            });
        }
    }
}

fn png_unfinished_result(
    allow_partial: bool,
    start: u64,
    last_good_position: u64,
    saw_ihdr: bool,
    saw_idat: bool,
    integrity_ok: bool,
) -> Option<CarveResult> {
    if !allow_partial || !saw_ihdr {
        return None;
    }

    let length = last_good_position.saturating_sub(start);
    if length < MIN_PARTIAL_IMAGE_BYTES {
        return None;
    }

    if saw_idat {
        Some(CarveResult::partial(
            length,
            if integrity_ok { 66 } else { 50 },
        ))
    } else {
        Some(CarveResult::corrupted(length, 40))
    }
}

fn png_damaged_result(
    allow_partial: bool,
    start: u64,
    last_good_position: u64,
    saw_ihdr: bool,
    saw_idat: bool,
) -> Option<CarveResult> {
    if !allow_partial || !saw_ihdr {
        return None;
    }

    let length = last_good_position.saturating_sub(start);
    if length < MIN_PARTIAL_IMAGE_BYTES {
        return None;
    }

    Some(CarveResult::corrupted(
        length,
        if saw_idat { 48 } else { 38 },
    ))
}

fn verify_png_crc<R: Read + Seek>(
    reader: &mut R,
    data_offset: u64,
    chunk_type: &[u8],
    data_length: u64,
) -> Option<bool> {
    reader.seek(SeekFrom::Start(data_offset)).ok()?;
    let mut hasher = Hasher::new();
    hasher.update(chunk_type);
    let mut remaining = data_length;
    let mut buffer = vec![0_u8; 1024 * 1024];

    while remaining > 0 {
        let wanted = remaining.min(buffer.len() as u64) as usize;
        reader.read_exact(&mut buffer[..wanted]).ok()?;
        hasher.update(&buffer[..wanted]);
        remaining -= wanted as u64;
    }

    let mut crc = [0_u8; 4];
    reader.read_exact(&mut crc).ok()?;
    Some(hasher.finalize() == u32::from_be_bytes(crc))
}

fn carve_webp<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    max_file_size: u64,
) -> Option<CarveResult> {
    let header = read_at(reader, start, 12)?;
    if &header[..4] != b"RIFF" || &header[8..12] != b"WEBP" {
        return None;
    }
    let riff_length = u32::from_le_bytes(header[4..8].try_into().ok()?) as u64;
    let total_length = riff_length.checked_add(8)?;
    if !(12..=max_file_size).contains(&total_length) {
        return None;
    }
    Some(CarveResult::complete(total_length, 95))
}

fn carve_bmp<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    max_file_size: u64,
) -> Option<CarveResult> {
    let header = read_at(reader, start, 18)?;
    if &header[..2] != b"BM" {
        return None;
    }
    let total_length = u32::from_le_bytes(header[2..6].try_into().ok()?) as u64;
    let pixel_offset = u32::from_le_bytes(header[10..14].try_into().ok()?) as u64;
    let dib_size = u32::from_le_bytes(header[14..18].try_into().ok()?) as u64;
    if total_length < 26
        || total_length > max_file_size
        || pixel_offset < 18
        || pixel_offset >= total_length
        || dib_size < 12
    {
        return None;
    }
    Some(CarveResult::complete(total_length, 90))
}

fn carve_gif<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    max_file_size: u64,
) -> Option<CarveResult> {
    let header = read_at(reader, start, 13)?;
    if &header[..6] != b"GIF87a" && &header[..6] != b"GIF89a" {
        return None;
    }

    let mut position = start + 13;
    let limit = start.checked_add(max_file_size)?;
    if header[10] & 0x80 != 0 {
        let table_size = 3_u64 * (1_u64 << ((header[10] & 0x07) + 1));
        position = position.checked_add(table_size)?;
    }

    loop {
        if position >= limit {
            return None;
        }
        match read_u8_at(reader, position)? {
            0x3B => return Some(CarveResult::complete(position + 1 - start, 96)),
            0x2C => {
                let descriptor = read_at(reader, position + 1, 9)?;
                position += 10;
                if descriptor[8] & 0x80 != 0 {
                    position += 3_u64 * (1_u64 << ((descriptor[8] & 0x07) + 1));
                }
                position += 1;
                position = skip_gif_sub_blocks(reader, position, limit)?;
            }
            0x21 => {
                position += 2;
                position = skip_gif_sub_blocks(reader, position, limit)?;
            }
            _ => return None,
        }
    }
}

fn skip_gif_sub_blocks<R: Read + Seek>(
    reader: &mut R,
    mut position: u64,
    limit: u64,
) -> Option<u64> {
    loop {
        if position >= limit {
            return None;
        }
        let size = read_u8_at(reader, position)? as u64;
        position += 1;
        if size == 0 {
            return Some(position);
        }
        position = position.checked_add(size)?;
        if position > limit {
            return None;
        }
    }
}

fn carve_heic<R: Read + Seek>(
    reader: &mut R,
    start: u64,
    max_file_size: u64,
) -> Option<CarveResult> {
    let first = read_at(reader, start, 16)?;
    let first_size = u32::from_be_bytes(first[..4].try_into().ok()?) as u64;
    if &first[4..8] != b"ftyp"
        || !(16..=1024 * 1024).contains(&first_size)
        || first_size > max_file_size
        || !is_heic_brand(&first[8..12])
    {
        return None;
    }

    let ftyp = read_at(reader, start, first_size as usize)?;
    let has_compatible_brand = ftyp[8..].chunks_exact(4).any(is_heic_brand);
    if !has_compatible_brand {
        return None;
    }

    let mut position = start + first_size;
    let limit = start.checked_add(max_file_size)?;
    let mut box_count = 1_u32;
    let mut saw_media_data = false;

    while position + 8 <= limit {
        let header = match read_at(reader, position, 8) {
            Some(header) => header,
            None => break,
        };
        let mut box_size = u32::from_be_bytes(header[..4].try_into().ok()?) as u64;
        let box_type = &header[4..8];
        let header_size = if box_size == 1 {
            let extended = read_at(reader, position + 8, 8)?;
            box_size = u64::from_be_bytes(extended.try_into().ok()?);
            16
        } else {
            8
        };

        if box_size == 0
            || box_size < header_size
            || position + box_size > limit
            || !box_type
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == b' ')
        {
            break;
        }
        saw_media_data |= box_type == b"mdat";
        box_count += 1;
        position += box_size;
    }

    (box_count >= 2 && saw_media_data).then_some(CarveResult::complete(position - start, 75))
}

fn is_heic_brand(brand: &[u8]) -> bool {
    matches!(
        brand,
        b"heic"
            | b"heix"
            | b"hevc"
            | b"hevx"
            | b"heim"
            | b"heis"
            | b"mif1"
            | b"msf1"
            | b"avif"
            | b"avis"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(kind);
        chunk.extend_from_slice(data);
        let mut hasher = Hasher::new();
        hasher.update(kind);
        hasher.update(data);
        chunk.extend_from_slice(&hasher.finalize().to_be_bytes());
        chunk
    }

    #[test]
    fn identifies_supported_signatures() {
        assert_eq!(detect_format(b"\xff\xd8\xff\xe0"), Some(ImageFormat::Jpeg));
        assert_eq!(detect_format(b"\x89PNG\r\n\x1a\n"), Some(ImageFormat::Png));
        assert_eq!(
            detect_format(b"RIFF\x04\0\0\0WEBP"),
            Some(ImageFormat::Webp)
        );
        assert_eq!(detect_format(b"GIF89a"), Some(ImageFormat::Gif));
        assert_eq!(detect_format(b"BMwhatever"), Some(ImageFormat::Bmp));
        assert_eq!(
            detect_format(b"\0\0\0\x18ftypheic"),
            Some(ImageFormat::Heic)
        );
    }

    #[test]
    fn parses_png_until_iend_and_checks_crc() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&png_chunk(b"IHDR", &[0; 13]));
        png.extend_from_slice(&png_chunk(b"IDAT", &[]));
        png.extend_from_slice(&png_chunk(b"IEND", &[]));
        let mut cursor = Cursor::new(png.clone());
        let result = carve_png(&mut cursor, 0, 1024, false).unwrap();
        assert_eq!(result.length, png.len() as u64);
        assert_eq!(result.status, CandidateStatus::Found);
    }

    #[test]
    fn jpeg_parser_ignores_an_embedded_thumbnail_footer_before_scan_data() {
        let jpeg = vec![
            0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x06, 0xFF, 0xD8, 0xFF, 0xD9, 0xFF, 0xDA, 0x00, 0x02,
            0x11, 0x22, 0xFF, 0x00, 0x33, 0xFF, 0xD9,
        ];
        let mut cursor = Cursor::new(jpeg.clone());
        let result = carve_jpeg(&mut cursor, 0, 1024, false).unwrap();
        assert_eq!(result.length, jpeg.len() as u64);
        assert_eq!(result.status, CandidateStatus::Found);
    }

    #[test]
    fn normal_jpeg_without_footer_is_ignored() {
        let jpeg = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22,
            0x33, 0x44, 0x55, 0x66,
        ];
        let mut cursor = Cursor::new(jpeg);
        assert!(carve_jpeg(&mut cursor, 0, 1024, false).is_none());
    }

    #[test]
    fn deep_jpeg_without_footer_becomes_partial() {
        let jpeg = vec![
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xDA, 0x00, 0x02, 0x11, 0x22,
            0x33, 0x44, 0x55, 0x66,
        ];
        let mut cursor = Cursor::new(jpeg.clone());
        let result = carve_jpeg(&mut cursor, 0, 1024, true).unwrap();
        assert_eq!(result.length, jpeg.len() as u64);
        assert_eq!(result.status, CandidateStatus::Partial);
    }

    #[test]
    fn deep_png_without_iend_becomes_partial() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&png_chunk(b"IHDR", &[0; 13]));
        png.extend_from_slice(&png_chunk(b"IDAT", &[1, 2, 3]));
        let mut cursor = Cursor::new(png.clone());
        let result = carve_png(&mut cursor, 0, 1024, true).unwrap();
        assert_eq!(result.length, png.len() as u64);
        assert_eq!(result.status, CandidateStatus::Partial);
    }

    #[test]
    fn parses_gif_blocks_instead_of_stopping_on_any_trailer_byte() {
        let gif = vec![
            b'G', b'I', b'F', b'8', b'9', b'a', 1, 0, 1, 0, 0, 0, 0, 0x21, 0xFE, 2, 0x3B, 0x41, 0,
            0x3B,
        ];
        let mut cursor = Cursor::new(gif.clone());
        let result = carve_gif(&mut cursor, 0, 1024).unwrap();
        assert_eq!(result.length, gif.len() as u64);
    }

    #[test]
    fn parses_bmp_size_and_pixel_offset() {
        let mut bmp = vec![0_u8; 64];
        bmp[..2].copy_from_slice(b"BM");
        bmp[2..6].copy_from_slice(&64_u32.to_le_bytes());
        bmp[10..14].copy_from_slice(&54_u32.to_le_bytes());
        bmp[14..18].copy_from_slice(&40_u32.to_le_bytes());
        let mut cursor = Cursor::new(bmp);
        let result = carve_bmp(&mut cursor, 0, 1024).unwrap();
        assert_eq!(result.length, 64);
    }

    #[test]
    fn parses_heic_top_level_boxes() {
        let mut heic = Vec::new();
        heic.extend_from_slice(&24_u32.to_be_bytes());
        heic.extend_from_slice(b"ftyp");
        heic.extend_from_slice(b"heic");
        heic.extend_from_slice(&0_u32.to_be_bytes());
        heic.extend_from_slice(b"heic");
        heic.extend_from_slice(b"mif1");
        heic.extend_from_slice(&12_u32.to_be_bytes());
        heic.extend_from_slice(b"mdat");
        heic.extend_from_slice(&[1, 2, 3, 4]);
        let mut cursor = Cursor::new(heic.clone());
        let result = carve_heic(&mut cursor, 0, 1024).unwrap();
        assert_eq!(result.length, heic.len() as u64);
    }
}
