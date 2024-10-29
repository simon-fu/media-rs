use std::fmt;

use super::error::RtpError;


pub struct RefRtcpHeader<'a> {
    buf: &'a [u8],
}

impl<'a> RefRtcpHeader<'a> {
    
    pub const MIN_LEN: usize = 8;

    pub const PT_MIN: u8 = 64 + 128; // 192
    pub const PT_MAX: u8 = 95 + 128; // 223

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
    pub fn r_count(&self) -> u8 {
        self.buf[0] & 0b0001_1111
    }

    #[inline]
    pub fn payload_type(&self) -> u8 {
        self.buf[1]
    }

    #[inline]
    pub fn words_minus_one(&self) -> u16 {
        u16::from_be_bytes([self.buf[2], self.buf[3]])
    }

    #[inline]
    pub fn ssrc(&self) -> u32 {
        u32::from_be_bytes([self.buf[4], self.buf[5], self.buf[6], self.buf[7]])
    }
}

impl<'a> TryFrom<&'a [u8]> for RefRtcpHeader<'a> {
    type Error = RtpError;

    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        if buf.len() < Self::MIN_LEN {
            return Err(RtpError::NotEnoughBuffer {
                expect: Self::MIN_LEN,
                actual: buf.len(),
                origin: "Rtcp header length",
            });
        }

        let header = Self::new(buf);

        if header.version() != 2 {
            return Err(RtpError::UnknownVersion(header.version()));
        }

        if header.payload_type() < Self::PT_MIN || header.payload_type() > Self::PT_MAX {
            return Err(RtpError::UnknownPayloadType(header.payload_type()));
        }

        Ok(header)
    }
}



pub struct RefRtcpPacket<'a> {
    buf: &'a [u8],
}

impl<'a> RefRtcpPacket<'a>  {

    // pub fn iter_from(buf: &'a [u8]) -> impl Iterator< Item = Result<Self, RtpError> > + 'a {
    //     RefRtcpIter {buf}
    // }

    // pub fn is_valid(buf: &'a [u8]) -> bool {
    //     let mut iter = TryRtcpIter {buf};
    //     let r = iter.try_for_each(|item| item.map(|_v| ()));
    //     r.is_ok()
    // }

    pub fn uncheck(buf: &'a [u8]) -> RefRtcpPacket<'_> {
        Self { buf }
    }

    #[inline]
    pub fn inner(&self) -> &'a [u8] {
        self.buf
    }
    
    #[inline]
    pub fn header(&self) -> RefRtcpHeader<'a> {
        RefRtcpHeader {
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

    pub fn payload_offset(&self) -> usize {
        RefRtcpHeader::MIN_LEN
    }

    pub fn payload(&self) -> &'a [u8] {
        
        let pad = self.padding().unwrap_or(0) as usize;

        &self.buf[self.payload_offset()..self.buf.len() - pad]
    }

    pub fn packet_len(&self) -> usize {
        ((self.header().words_minus_one() + 1) * 4) as usize
    }
}

impl<'a> TryFrom<&'a [u8]> for RefRtcpPacket<'a> {
    type Error = RtpError;

    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        let header = RefRtcpHeader::try_from(buf)?;

        let me = Self{ buf };

        let payload_offset = me.payload_offset();

        if header.padding_flag() {
            let post_header_bytes =  buf.len() - payload_offset;
            
            if post_header_bytes == 0 {
                return Err(RtpError::NotEnoughBuffer {
                    expect:  payload_offset,
                    actual: buf.len() - 1,
                    origin: "Rtcp padding field",
                });
            }
            let pad_len = me.parse_padding_len()?;

            if payload_offset + pad_len as usize > buf.len() {
                return Err(RtpError::NotEnoughBuffer {
                    expect:  payload_offset + pad_len as usize,
                    actual: buf.len() ,
                    origin: "Rtcp padding length",
                });
            }
        }

        let packet_len = me.packet_len();
        if buf.len() < packet_len {
            return Err(RtpError::NotEnoughBuffer {
                expect: packet_len,
                actual: buf.len(),
                origin: "Rtcp packet length",
            });
        }

        Ok(me)
    }
}


impl<'a> fmt::Debug for RefRtcpPacket<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let header = self.header();

        f.debug_struct("Rtcp")
            .field("version", &header.version())
            .field("padding", &self.padding())
            .field("r_count", &header.r_count())
            .field("payload_type", &header.payload_type())
            .field("ssrc", &header.ssrc())
            .field("payload_length", &self.payload().len())
            .finish()
    }
}

impl<'a> fmt::Display for RefRtcpPacket<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let header = self.header();
        write!(f, 
            "ssrc {}, pt {}, rc {}", 
            header.ssrc(),
            header.payload_type(), 
            header.r_count(),
        )?;

        write!(f, 
            ", body {}", 
            self.payload().len(),
        )?;

        Ok(())
    }
}




pub struct RefRtcpPackets<'a> {
    buf: &'a [u8],
}

impl<'a> RefRtcpPackets<'a>  {

    pub fn uncheck_iter(&'a self) -> impl Iterator< Item = RefRtcpPacket<'a> > + 'a {
        RtcpUncheckIter {
            buf: self.buf
        }
    }

    fn try_iter(&'a self) -> impl Iterator< Item = Result<RefRtcpPacket, RtpError> > + 'a {
        RtcpTryIter {
            buf: self.buf,
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for RefRtcpPackets<'a> {
    type Error = RtpError;

    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        let me = Self {buf};

        let _r = me.try_iter().try_for_each(|item| item.map(|_v| ()))?;

        Ok(me)
    }
}

pub struct RtcpTryIter<'a> {
    buf: &'a [u8],
}

impl<'a> Iterator for RtcpTryIter<'a> {
    type Item = Result<RefRtcpPacket<'a>, RtpError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.buf.is_empty() {
            match RefRtcpPacket::try_from(self.buf) {
                Ok(v) => {
                    self.buf = &self.buf[v.packet_len()..];
                    Some(Ok(v))
                },
                Err(e) => {
                    self.buf = &[];
                    Some(Err(e))
                },
            }
        } else {
            None
        }
    }
}

pub struct RtcpUncheckIter<'a> {
    buf: &'a [u8],
}

impl<'a> Iterator for RtcpUncheckIter<'a> {
    type Item = RefRtcpPacket<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.buf.is_empty() {
            let item = RefRtcpPacket::uncheck(self.buf);
            self.buf = &self.buf[item.packet_len()..];
            Some(item)
        } else {
            None
        }
    }
}

impl<'a> fmt::Debug for RefRtcpPackets<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str("[ ")?;
        let mut iter = self.uncheck_iter();

        if let Some(packet) = iter.next() {
            write!(f, "{packet:?}")?;
        }

        for packet in iter {
            write!(f, ", {packet:?}")?;
        }
        
        f.write_str(" ]")?;

        Ok(())
    }
}

impl<'a> fmt::Display for RefRtcpPackets<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str("[ ")?;
        let mut iter = self.uncheck_iter();

        if let Some(packet) = iter.next() {
            write!(f, "{{ {packet} }}")?;
        }

        for packet in iter {
            write!(f, ", {{ {packet} }}")?;
        }
        
        f.write_str(" ]")?;

        Ok(())
    }
}


// pub fn check_is_rtcp(data: &[u8]) -> bool {
//     // Check the RTP payload type.  If 63 < payload type < 96, it's RTCP.
//     // For additional details, see http://tools.ietf.org/html/rfc5761.
//     // https://blog.csdn.net/ciengwu/article/details/78024121#:~:text=rtp%E4%B8%8Ertcp%E5%8D%8F%E8%AE%AE%E5%A4%B4,%E5%B0%B1%E5%8F%98%E4%B8%BA%E4%BA%8672~78%E3%80%82

//     if data.len() < 2 {
//         return false;
//     }
//     let pt = data[1] & 0x7F;
//     return (63 < pt) && (pt < 96);
// }
