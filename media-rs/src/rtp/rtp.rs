use std::fmt::{self, Write};
use bytes::BufMut;

use super::{error::RtpError, extension::{ExtFormat, ExtIter, WriteExtFns}, Seq, Timestamp};


pub struct RefRtpHeader<'a> {
    buf: &'a [u8],
}

impl<'a> RefRtpHeader<'a> {

    pub const MIN_LEN: usize = 12;

    pub fn new(buf: &'a [u8]) -> Self {
        assert!(buf.len() >= Self::MIN_LEN);
        Self { buf }
    }

    #[inline]
    pub fn version(&self) -> u8 {
        (self.buf[0] & 0b1100_0000) >> 6
    }

    #[inline]
    pub fn padding_flag(&self) -> bool {
        (self.buf[0] & 0b0010_0000) != 0
    }

    #[inline]
    pub fn extension_flag(&self) -> bool {
        (self.buf[0] & 0b0001_0000) != 0
    }

    #[inline]
    pub fn mark_flag(&self) -> bool {
        (self.buf[1] & 0b1000_0000) != 0
    }

    #[inline]
    pub fn payload_type(&self) -> u8 {
        self.buf[1] & 0b0111_1111
    }
    
    #[inline]
    pub fn seq(&self) -> Seq {
        Seq((self.buf[2] as u16) << 8 | (self.buf[3] as u16))
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        Timestamp(u32::from_be_bytes([self.buf[4], self.buf[5], self.buf[6], self.buf[7]]))
        // (self.buf[4] as u32) << 24
        //     | (self.buf[5] as u32) << 16
        //     | (self.buf[6] as u32) << 8
        //     | (self.buf[7] as u32)
    }

    #[inline]
    pub fn ssrc(&self) -> u32 {
        u32::from_be_bytes([self.buf[8], self.buf[9], self.buf[10], self.buf[11]])
        // (self.buf[8] as u32) << 24
        //     | (self.buf[9] as u32) << 16
        //     | (self.buf[10] as u32) << 8
        //     | (self.buf[11] as u32)
    }

    #[inline]
    pub fn csrc_count(&self) -> u8 {
        self.buf[0] & 0b0000_1111
    }

    /// 12 + csrc   
    /// 
    #[inline]
    pub fn header_end(&self) -> usize {
        Self::MIN_LEN + 4 * (self.csrc_count() as usize) 
        // Self::MIN_LEN + (4 * self.csrc_count()) as usize
    }

}

impl<'a> TryFrom<&'a [u8]> for RefRtpHeader<'a> {
    type Error = RtpError;

    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        /* 
            From: https://tools.ietf.org/html/draft-ietf-avtcore-rfc5764-mux-fixes
            is rtp/rtcp?
                1) len >= 12
                2) version = 2
                3) buf[0] > 127 && buf[0] < 192

                        +----------------+
                        |        [0..3] -+--> forward to STUN
                        |                |
                        |      [16..19] -+--> forward to ZRTP
                        |                |
            packet -->  |      [20..63] -+--> forward to DTLS
                        |                |
                        |      [64..79] -+--> forward to TURN Channel
                        |                |
                        |    [128..191] -+--> forward to RTP/RTCP
                        +----------------+
        */

        if buf.len() < Self::MIN_LEN {
            return Err(RtpError::NotEnoughBuffer {
                expect: Self::MIN_LEN,
                actual: buf.len(),
                origin: "Rtp header length",
            });
        }

        {
            let first = buf[0];
            if !(first > 127 && first < 192) {
                return Err(RtpError::UnknownFirst(first));
            }
        }

        let header = Self::new(buf);

        if header.version() != 2 {
            return Err(RtpError::UnknownVersion(header.version()));
        }

        Ok(header)
    }
}



#[derive(Clone, Copy)]
pub struct RefRtpPacket<'a> {
    buf: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for RefRtpPacket<'a> {
    type Error = RtpError;

    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::parse(buf)
    }
}

impl<'a> RefRtpPacket<'a> {

    const EXTENSION_HEADER_LEN: usize = 4;

    pub fn parse(buf: &'a [u8]) -> Result<RefRtpPacket<'_>, RtpError> {
        let header = RefRtpHeader::try_from(buf)?;

        let me = Self{ buf };

        if header.extension_flag() {
            let extension_start = header.header_end() + Self::EXTENSION_HEADER_LEN;
            if extension_start > buf.len() {
                return Err(RtpError::NotEnoughBuffer {
                    expect: extension_start,
                    actual: buf.len(),
                    origin: "Rtp extension start",
                });
            }

            let extension_end = extension_start + me.extension_len();
            if extension_end > buf.len() {
                return Err(RtpError::NotEnoughBuffer {
                    expect: extension_end,
                    actual: buf.len(),
                    origin: "Rtp extension end",
                });
            }

            let (ext_fmt, ext_buf) = me.extension_uncheck(header.header_end());
            let ext_fmt = ExtFormat::from_num(ext_fmt).map_err(|_e|RtpError::UnknownExtFormat(ext_fmt))?;
            ext_fmt.check(ext_buf)?;
        }

        let payload_offset = me.payload_offset();
        if payload_offset > buf.len() {
            return Err(RtpError::NotEnoughBuffer {
                expect:  payload_offset,
                actual: buf.len(),
                origin: "Rtp payload offset",
            });
        }

        if header.padding_flag() {
            let post_header_bytes =  buf.len() - payload_offset;
            
            if post_header_bytes == 0 {
                return Err(RtpError::NotEnoughBuffer {
                    expect:  payload_offset,
                    actual: buf.len() - 1,
                    origin: "Rtp padding field",
                });
            }
            let pad_len = me.parse_padding_len()?;

            if payload_offset + pad_len as usize > buf.len() {
                return Err(RtpError::NotEnoughBuffer {
                    expect:  payload_offset + pad_len as usize,
                    actual: buf.len() ,
                    origin: "Rtp padding length",
                });
            }
        }
        Ok(me)
    }

    pub fn uncheck(buf: &'a [u8]) -> RefRtpPacket<'_> {
        Self { buf }
    }

    #[inline]
    pub fn inner(&self) -> &'a [u8] {
        self.buf
    }
    
    #[inline]
    pub fn header(&self) -> RefRtpHeader<'a> {
        RefRtpHeader {
            buf: self.buf,
        }
    }
    
    #[inline]
    pub fn padding(&self) -> Option<u8> {
        if self.header().padding_flag() {
            Some(self.padding_len_uncheck())
        } else {
            None
        }
    }

    #[inline]
    fn padding_len_uncheck(&self) -> u8 {
        self.buf[self.buf.len() - 1]
    }

    fn parse_padding_len(&self) -> Result<u8, RtpError> {
        match self.padding_len_uncheck() {
            0 => Err(RtpError::InvalidPaddingLength(0)),
            l => Ok(l),
        }
    }
    
    pub fn csrc_iter(&self) -> impl Iterator<Item = u32> + '_ {
        let header = self.header();

        self.buf[RefRtpHeader::MIN_LEN..]
            .chunks(4)
            .take(header.csrc_count() as usize)
            .map(|b| (b[0] as u32) << 24 | (b[1] as u32) << 16 | (b[2] as u32) << 8 | (b[3] as u32))
    }

    
    pub fn payload_offset(&self) -> usize {
        let header = self.header();

        let offset = header.header_end();
        if header.extension_flag() {
            offset + Self::EXTENSION_HEADER_LEN + self.extension_len()
        } else {
            offset
        }
    }

    pub fn payload(&self) -> &'a [u8] {
        
        let pad = self.padding().unwrap_or(0) as usize;

        &self.buf[self.payload_offset()..self.buf.len() - pad]
    }

    pub fn extension_iter(&self) -> Option<ExtIter<'a>> {
        match self.extension() {
            Some((ext_fmt, ext_buf)) => {
                Some(ExtFormat::from_num_uncheck(ext_fmt).iter(ext_buf))
            },
            None => None,
        }
    }
    
    fn extension(&self) -> Option<(u16, &'a [u8])> {
        let header = self.header();
        if header.extension_flag() {
            Some(self.extension_uncheck(header.header_end()))
        } else {
            None
        }
    }

    #[inline]
    fn extension_uncheck(&self, offset: usize) -> (u16, &'a [u8]) {
        let ext_fmt = (self.buf[offset] as u16) << 8 | (self.buf[offset + 1] as u16);
        let start = offset + 4;
        (ext_fmt, &self.buf[start..start + self.extension_len()])
    }

    #[inline]
    fn extension_len(&self) -> usize {
        let offset = self.header().header_end();

        4 * ((self.buf[offset + 2] as usize) << 8 | (self.buf[offset + 3] as usize))
    }
}


impl<'a> fmt::Debug for RefRtpPacket<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let header = self.header();

        f.debug_struct("Rtp")
            .field("version", &header.version())
            .field("padding", &self.padding())
            .field("extension", &self.extension().map(|(id, _)| id))
            .field("csrc_count", &header.csrc_count())
            .field("mark", &header.mark_flag())
            .field("payload_type", &header.payload_type())
            .field("seq", &header.seq())
            .field("timestamp", &header.timestamp())
            .field("ssrc", &header.ssrc())
            .field("payload_length", &self.payload().len())
            .finish()
    }
}

impl<'a> fmt::Display for RefRtpPacket<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let header = self.header();
        write!(f, 
            "ssrc {}, pt {}, seq {}, ts {}", 
            header.ssrc(),
            header.payload_type(), 
            header.seq().0, 
            header.timestamp().0,
        )?;

        {
            let mut iter = self.csrc_iter();
            if let Some(csrc) = iter.next() {
                write!(f, ", csrc[")?;
                write!(f, "{csrc}")?;
                for csrc in iter {
                    write!(f, ", {csrc}")?;
                }
                f.write_char(']')?;
            }
        }

        {
            write!(f, ", ext[")?;

            if let Some(mut iter) = self.extension_iter() {
                if let Some((id, _buf)) = iter.next() {
                    write!(f, "{id}")?;
                    for (id, _buf) in iter {
                        write!(f, ", {id}")?;
                    }
                }
            }

            f.write_char(']')?;
        }

        write!(f, 
            ", body {}", 
            self.payload().len(),
        )?;

        if header.mark_flag() {
            f.write_str(", m 1")?;
        }

        Ok(())
    }
}



pub struct RtpBuilder<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> RtpBuilder<'a> {

    pub fn from_basic<CI>(
        buf: &'a mut [u8],
        mark_flag: bool,
        payload_type: u8,
        seq: Seq,
        timestamp: Timestamp,
        ssrc: u32,
        csrc_iter: CI,
    ) -> Self 
    where 
        CI: Iterator<Item = u32> 
    {
        let len = build_header(buf, mark_flag, payload_type, seq, timestamp, ssrc, csrc_iter);
        Self {
            buf,
            len,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn extension_one(self, id: u8, ext: &[u8]) -> PayloadBuilder<'a> {
        let mut ext_builder = self.extension(ExtFormat::OneByte);
        ext_builder.write_ext(id, ext);
        ext_builder.payload_builder()
    }

    pub fn extension(mut self, ext_fmt: ExtFormat) -> ExtBuilder<'a> {
        {
            let mut buf = &mut self.buf[self.len..];
            buf.put_u16(ext_fmt as u16);
            buf.put_u16(0); // words
        }

        // buf[0]: version(2), padding(1), extension(1), cc(4) 
        self.buf[0] |= 0b00_0_1_0000 ; 

        let offset = self.len;
        self.len += 4;

        ExtBuilder {
            func: ext_fmt.build_fn(),
            // owner: self,
            total_len: self.len,
            buf: self.buf,
            offset,
        }
    }

    pub fn payload(self, payload: &[u8], padding: bool) -> usize {
        build_payload(self.buf, self.len, payload, padding)
    }

    pub fn payload_builder(self) -> PayloadBuilder<'a> {
        PayloadBuilder {
            buf: self.buf,
            total_len: self.len
        }
    }

}


pub struct ExtBuilder<'a> {
    func: WriteExtFns,
    // owner: &'a mut RtpBuilder<'a>,
    buf: &'a mut [u8],
    total_len: usize,
    offset: usize,
}

impl<'a> ExtBuilder<'a> {

    #[inline]
    pub fn write_ext(&mut self, id: u8, ext: &[u8]) {
        let buf = &mut self.buf[self.total_len..];

        let header_len = (self.func.begin_fn)(buf, id);
        (&mut buf[header_len..header_len+ext.len()]).copy_from_slice(ext);
        (self.func.end_fn)(buf, ext.len());

        self.total_len += header_len + ext.len();

        // let len = (self.func)(&mut self.buf[self.total_len..], id, ext);
        // self.total_len += len;
    }

    #[inline]
    pub fn ext<'b>(&'b mut self, id: u8) -> ExtItemBuilder<'b, 'a> {
        let offset = self.total_len;

        let header_len = (self.func.begin_fn)(&mut self.buf[offset..], id);

        ExtItemBuilder {
            parent: self,
            offset,
            header_len,
            item_body_len: 0,
        }
    }

    // #[inline]
    // pub fn last(mut self, id: u8, ext: &[u8], payload: &[u8], padding: bool) -> usize {
    //     self.write_ext(id, ext);
    //     self.payload(payload, padding)
    // }

    pub fn payload(mut self, payload: &[u8], padding: bool) -> usize {
        self.finish();
        build_payload(self.buf, self.total_len, payload, padding)
    }

    fn payload_builder(mut self) -> PayloadBuilder<'a> {
        self.finish();
        PayloadBuilder {
            buf: self.buf,
            total_len: self.total_len,
        }
    }

    fn finish(&mut self) {
        let len = self.total_len - self.offset  - 4;
        let words = (len + 3) / 4;

        // // no extensions
        // if words == 0 {
            
        //     self.total_len -= 4;
            
        //     // buf[0]: version(2), padding(1), extension(1), cc(4) 
        //     self.buf[0] &= 0b11_1_0_1111 ; 

        //     return;
        // }

        let padding_len = (words * 4) - len;
        if padding_len > 0 {
            let mut buf = &mut self.buf[self.total_len..];
            buf.put_bytes(0, padding_len);
            self.total_len += padding_len;
        }

        let mut buf = &mut self.buf[self.offset+2..];
        buf.put_u16(words as u16);
    }
}

pub struct ExtItemBuilder<'a, 'b> {
    parent: &'a mut ExtBuilder<'b>,
    offset: usize,
    header_len: usize,
    item_body_len: usize,
}

impl<'a, 'b> Drop for ExtItemBuilder<'a, 'b> {
    fn drop(&mut self) {
        let item_body_len  = self.item_body_len;
        (self.parent.func.end_fn)(self.header_buf(), item_body_len);

        self.parent.total_len += self.header_len + self.item_body_len;
    }
}

impl<'a, 'b> ExtItemBuilder<'a, 'b> {
    pub fn write_u16(&mut self, value: u16) -> &mut Self {
        self.tail_buf().put_u16(value);
        self.item_body_len += 2;
        self
    }

    pub fn write_slice(&mut self, value: &[u8]) {
        self.tail_buf().put(value);
        self.item_body_len += value.len();
    }

    fn header_buf(&mut self) -> &mut [u8] {
        let offset = self.offset;
        &mut self.parent.buf[offset..]
    }

    fn tail_buf(&mut self) -> &mut [u8] {
        let tail = self.tail();
        &mut self.parent.buf[tail..]
    }

    fn tail(&self) -> usize {
        self.offset + self.header_len + self.item_body_len
    }
}


pub struct PayloadBuilder<'a> {
    // owner: &'a mut RtpBuilder<'a>,
    buf: &'a mut [u8],
    total_len: usize,
} 

impl<'a> PayloadBuilder<'a> {
    #[inline]
    pub fn payload(self, payload: &[u8], padding: bool) -> usize {
        build_payload(self.buf, self.total_len, payload, padding)
    }
}

fn build_payload(buf: &mut [u8], mut total_len: usize,  payload: &[u8], padding: bool) -> usize {

    let mut ptr = &mut buf[total_len..];
    ptr.put_slice(payload);
    total_len += payload.len();

    if padding {
        let words = (total_len + 3) / 4;
        let padding_len = (words * 4) - total_len;
        if padding_len > 0 {
            ptr.put_bytes(0, padding_len-1);
            ptr.put_u8(padding_len as u8);
            total_len += padding_len;

            // buf[0]: version(2), padding(1), extension(1), cc(4) 
            buf[0] |= 0b00_1_0_0000 ; 
        }
    }

    total_len
}



#[inline]
pub fn build_header<CI>(
    buf: &mut [u8],
    mark_flag: bool,
    payload_type: u8,
    seq: Seq,
    timestamp: Timestamp,
    ssrc: u32,
    csrc_iter: CI,
) -> usize
where 
    // B: BufMut + Buf, 
    CI: Iterator<Item = u32> 
{
    let mut csrc_count = 0_u8;

    // buf[12-..]: csrc
    {
        let mut ptr = &mut buf[RefRtpHeader::MIN_LEN..];    
        for csrc in csrc_iter {
            ptr.put_u32(csrc);
            csrc_count = csrc_count + 1;
        }
    }
    
    {
        // buf[0]: version(2) = 2, padding(1), extension(1), cc(4) 
        buf[0] = 0b10_0_0_0000 | csrc_count; 

        // buf[1]: mark(1), payload_type(7)
        buf[1] = if mark_flag {
            0b1000_0000 | payload_type
        } else {
            0b0111_1111 & payload_type
        };

        // buf[2-3]: seq
        {
            let bytes = seq.0.to_be_bytes();
            buf[2] = bytes[0];
            buf[3] = bytes[1];
        }

        // buf[4-7]: timestamp
        {
            let bytes = timestamp.0.to_be_bytes();
            buf[4] = bytes[0];
            buf[5] = bytes[1];
            buf[6] = bytes[2];
            buf[7] = bytes[3];
        }

        // buf[8-11]: ssrc
        {
            let bytes = ssrc.to_be_bytes();
            buf[8] = bytes[0];
            buf[9] = bytes[1];
            buf[10] = bytes[2];
            buf[11] = bytes[3];
        }

        RefRtpHeader::MIN_LEN + 4 * (csrc_count as usize) 
    }

}


#[cfg(test)]
mod test {
    use crate::rtp::{extension::ExtFormat, Seq, Timestamp};

    use super::{RefRtpPacket, RtpBuilder, PayloadBuilder};

    #[derive(Debug, Clone)]
    struct Case {
        padding: bool,
        mark_flag: bool,
        payload_type: u8,
        seq: Seq,
        timestamp: Timestamp,
        ssrc: u32,
        csrc: Vec<u32>,
        ext_fmt: Option<ExtFormat>,
        exts: Vec<(u8, Vec<u8>)>,
        payload: Vec<u8>,
    }

    impl Default for Case {
        fn default() -> Self {
            Self {
                padding: true,
                mark_flag: true,
                payload_type: 111,
                seq: Seq::from(22),
                timestamp: 3333.into(),
                ssrc: 4444,
                csrc: vec![5555], 
                ext_fmt: Some(ExtFormat::OneByte),
                exts: vec![
                    (10, vec![7, 8_u8]),
                ],
                payload: vec![1, 2, 9, 8, 7_u8],
            }
        }
    }

    impl Case {
        fn build_and_check(&self, buf: &mut [u8]) {
            // let builder = RtpBuilder ::from_basic(
            //     buf, 
            //     self.mark_flag, 
            //     self.payload_type, 
            //     self.seq, 
            //     self.timestamp, 
            //     self.ssrc, 
            //     self.csrc.iter().map(|x|*x),
            // );

            // let packet_len = match self.ext_fmt {
            //     Some(ext_fmt) => {
            //         let mut builder = builder.extension(ext_fmt);
            //         for ext in self.exts.iter() {
            //             builder.write_ext(ext.0, &ext.1);
            //         }
            //         builder.payload(&self.payload[..], self.padding)
            //     },
            //     None => builder.payload(&self.payload[..], self.padding),
            // };
    
            // self.check(&buf[..packet_len]);

            self.build_with_and_check(buf, true);
            self.build_with_and_check(buf, false);

        }

        fn build_with_and_check(&self, buf: &mut [u8], whole_ext: bool) {
            let builder = self.rtp_builder(buf);

            let packet_len = self
            .build_rtp_extensions(builder, whole_ext)
            .payload(&self.payload[..], self.padding);

            self.check(&buf[..packet_len]);
        }

        fn rtp_builder<'a>(&self, buf: &'a mut [u8]) -> RtpBuilder<'a> {
            RtpBuilder ::from_basic(
                buf, 
                self.mark_flag, 
                self.payload_type, 
                self.seq, 
                self.timestamp, 
                self.ssrc, 
                self.csrc.iter().map(|x|*x),
            )
        }

        fn build_rtp_extensions<'a>(&self, builder: RtpBuilder<'a>, whole: bool)  -> PayloadBuilder<'a>{

            match self.ext_fmt {
                Some(ext_fmt) => {
                    let mut builder = builder.extension(ext_fmt);
                    if whole {
                        for ext in self.exts.iter() {
                            builder.write_ext(ext.0, &ext.1);
                        }
                    } else {

                        for ext in self.exts.iter() {
                            builder.ext(ext.0).write_slice(&ext.1);
                        }
                    }
                    
                    builder.payload_builder()
                },
                None => builder.payload_builder(),
            }
        }


        fn check(&self, buf: &[u8]) {
            let rtp = RefRtpPacket::parse(buf).unwrap();
            assert_eq!(rtp.header().version(), 2);

            if !self.padding {
                assert_eq!(rtp.header().padding_flag(), false);
            } else {
                if self.payload.len() % 4 == 0 {
                    assert_eq!(rtp.header().padding_flag(), false);
                } else {
                    assert_eq!(rtp.header().padding_flag(), true);
                }
            }

            assert_eq!(rtp.header().mark_flag(), self.mark_flag);
            assert_eq!(rtp.header().payload_type(), self.payload_type);
            assert_eq!(rtp.header().seq(), self.seq);
            assert_eq!(rtp.header().timestamp(), self.timestamp);
            assert_eq!(rtp.header().ssrc(), self.ssrc);

            {
                assert!(rtp.csrc_iter().eq(
                        self.csrc.iter().map(|x|*x)
                ));
            }
            
            if self.ext_fmt.is_some() {
                assert!(rtp.extension_iter().unwrap().eq(
                    self.exts.iter().map(|x|(x.0, x.1.as_slice()))    
                ));
            } else {
                assert!(rtp.extension_iter().is_none());
            }
            
            assert_eq!(rtp.payload(), &self.payload[..]);
        }
    }

    #[test]
    fn test_build_rtp() {
        let mut buf = vec![0_u8; 1700];

        // has extension
        {
            let mut caze = Case::default();
            caze.build_and_check(&mut buf);
    
            // extension onebyte format
            caze.ext_fmt = Some(ExtFormat::OneByte);
            caze.build_and_check(&mut buf);

            // extension twobyte format
            caze.ext_fmt = Some(ExtFormat::TwoByte);
            caze.build_and_check(&mut buf);


            // empty extension 
            caze.ext_fmt = Some(ExtFormat::OneByte);
            caze.exts = vec![];
            caze.build_and_check(&mut buf);
    
            // no extension 
            caze.ext_fmt = None;
            caze.build_and_check(&mut buf);
        }

        // payload padding
        {
            let mut caze = Case::default();

            for len in 1..=8 {
                caze.payload = vec![0; len];
                caze.build_and_check(&mut buf);
            }
        }

    }

}
