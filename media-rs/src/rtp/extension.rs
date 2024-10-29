//! https://datatracker.ietf.org/doc/html/rfc5285
//! https://datatracker.ietf.org/doc/html/rfc8285
//! 

use super::error::RtpError;


#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExtFormat {

    OneByte = Self::ONE_BYTE,
    
    TwoByte = Self::TWO_BYTE,
}

impl ExtFormat {
    const ONE_BYTE: u16 = 0xBEDE;
    const TWO_BYTE: u16 = 0x1000;

    pub fn from_num(num: u16) -> Result<Self, ()> {
        match num {
            Self::ONE_BYTE => Ok(Self::OneByte),
            Self::TWO_BYTE => Ok(Self::TwoByte),
            _ => Err(())
        }
    }

    pub(super) fn from_num_uncheck(num: u16) -> Self {
        match Self::from_num(num) {
            Ok(me) => me,
            Err(_e) => unreachable!("unexpect RTP extension format [{num}]")
        }
    }

    pub(super) fn check(&self, buf: &[u8]) -> Result<(), RtpError> {
        match self {
            Self::OneByte => check_ext(buf, parse_onebyte),
            Self::TwoByte => check_ext(buf, parse_twobyte),
        }
    }

    pub(super) fn iter<'a>(&self, buf: &'a [u8]) -> ExtIter<'a> {
        match self {
            Self::OneByte => ExtIter(buf, parse_onebyte_uncheck),
            Self::TwoByte => ExtIter(buf, parse_twobyte_uncheck),
        }
        
    }
}

type ParseIdFn = fn(buf: &[u8]) -> Result<Option<(&[u8], u8, usize)>, RtpError>;
type ParseIdUncheckFn = fn(buf: &[u8]) -> Option<(&[u8], u8, usize)>;

fn check_ext(mut buf: &[u8], f: ParseIdFn) -> Result<(), RtpError> {
    while !buf.is_empty() {

        if buf[0] == 0 {
            // padding
            buf = &buf[1..];
            continue;
        }


        let Some((next, _id, len)) = f(buf)? else {
            break;
        };

        buf = next;

        if buf.len() < len {
            return Err(RtpError::NotEnoughBuffer {
                expect: len,
                actual: buf.len(),
                origin: "OneByte ext body length",
            });
        }

        // let ext_buf = &buf[..len];
        buf = &buf[len..];
    }
    Ok(())
}

fn parse_onebyte (buf: &[u8]) -> Result<Option<(&[u8], u8, usize)>, RtpError> {
    Ok(parse_onebyte_uncheck(buf))
}

fn parse_onebyte_uncheck(buf: &[u8]) -> Option<(&[u8], u8, usize)> {
    let id = buf[0] >> 4;
    let len = (buf[0] & 0xf) as usize + 1;
    if id != 15 {
        Some((&buf[1..], id, len))
    } else {
        None
    }
}

fn parse_twobyte (buf: &[u8]) -> Result<Option<(&[u8], u8, usize)>, RtpError> {
    if buf.len() < 2 {
        return Err(RtpError::NotEnoughBuffer {
            expect: 2,
            actual: buf.len(),
            origin: "TwoByte ext header length",
        });
    }

    Ok(parse_twobyte_uncheck(buf))
}

fn parse_twobyte_uncheck(buf: &[u8]) -> Option<(&[u8], u8, usize)> {
    let id = buf[0];
    let len = buf[1] as usize;
    Some((&buf[2..], id, len))
}

pub struct ExtIter<'a>(&'a [u8], ParseIdUncheckFn);

impl<'a> Iterator for ExtIter<'a> {
    type Item = (u8, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        let buf = &mut self.0;

        while !buf.is_empty() {

            if buf[0] == 0 {
                // padding
                *buf = &buf[1..];
                continue;
            }
    
            let Some((next, id, len)) = self.1(buf) else {
                break;
            };
    
            let ext_buf = &next[..len];
            *buf = &next[len..];
            return Some((id, ext_buf))
        }
        None
    }
}


// pub struct OneByteIter<'a>(&'a [u8]);

// impl<'a> Iterator for OneByteIter<'a> {
//     type Item = Result<(u8, &'a [u8]), RtpError>;

//     fn next(&mut self) -> Option<Self::Item> {

//         self.0 = skip_padding(self.0);

//         if self.0.is_empty() {
//             return None
//         }

//         let buf = &mut self.0;
        
//         let id = buf[0] >> 4;
//         let len = (buf[0] & 0xf) as usize + 1;
//         *buf = &buf[1..];

//         if id == 15 {
//             /* 
//                 See 4.2.  One-Byte Header
//                 The local identifier value 15 is reserved for a future extension and
//                 MUST NOT be used as an identifier.  If the ID value 15 is
//                 encountered, its length field MUST be ignored, processing of the
//                 entire extension MUST terminate at that point, and only the extension
//                 elements present prior to the element with ID 15 SHOULD be
//                 considered.
//             */

//             *buf= &[];
//             return None
//         }

//         if buf.len() < len {
//             *buf= &[];
//             return Some(Err(RtpError::NotEnoughBuffer {
//                 expect: len,
//                 actual: buf.len(),
//                 origin: "OneByte ext body length",
//             }));
//         }

//         let ext_buf = &buf[..len];
//         *buf = &buf[len..];
//         return Some(Ok((id, ext_buf)))
        
//     }
// }


// pub struct TwoByteIter<'a>(&'a [u8]);

// impl<'a> Iterator for TwoByteIter<'a> {
//     type Item = Result<(u8, &'a [u8]), RtpError>;

//     fn next(&mut self) -> Option<Self::Item> {

//         self.0 = skip_padding(self.0);

//         if self.0.is_empty() {
//             return None
//         }

//         let buf = &mut self.0;

//         if buf.len() < 2 {
//             // Not enough buffer  
//             *buf= &[];

//             return Some(Err(RtpError::NotEnoughBuffer {
//                 expect: 2,
//                 actual: buf.len(),
//                 origin: "TwoByte ext header length",
//             }));
//         }
        
//         let id = buf[0];
//         let len = buf[1] as usize;
//         *buf = &buf[2..];

//         if buf.len() < len {
//             *buf= &[];
//             return Some(Err(RtpError::NotEnoughBuffer {
//                 expect: len,
//                 actual: buf.len(),
//                 origin: "TwoByte ext body length",
//             }));
//         }

//         let ext_buf = &buf[..len];
//         *buf = &buf[len..];
//         return Some(Ok((id, ext_buf)))
        
//     }
// }



// #[inline]
// fn skip_padding(mut buf: &[u8]) -> &[u8] {
//     while !buf.is_empty() {
//         if buf[0] != 0 {
//             break;
//         }
//         buf = &buf[1..];
//     }
//     buf
// }
