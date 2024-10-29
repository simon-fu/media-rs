use std::fmt::{self, Write};
use super::{error::RtpError, extension::{ExtFormat, ExtIter}, Seq};


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
    pub fn timestamp(&self) -> u32 {
        u32::from_be_bytes([self.buf[4], self.buf[5], self.buf[6], self.buf[7]])
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
        Self::MIN_LEN + (4 * self.csrc_count()) as usize
    }

}

impl<'a> TryFrom<&'a [u8]> for RefRtpHeader<'a> {
    type Error = RtpError;

    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        if buf.len() < Self::MIN_LEN {
            return Err(RtpError::NotEnoughBuffer {
                expect: Self::MIN_LEN,
                actual: buf.len(),
                origin: "Rtp header length",
            });
        }

        let header = Self::new(buf);

        if header.version() != 2 {
            return Err(RtpError::UnknownVersion(header.version()));
        }

        Ok(header)
    }
}



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
        let id = (self.buf[offset] as u16) << 8 | (self.buf[offset + 1] as u16);
        let start = offset + 4;
        (id, &self.buf[start..start + self.extension_len()])
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
            header.timestamp(),
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

