
#[derive(Debug)]
pub enum RtpError {
    NotEnoughBuffer {
        expect: usize,
        actual: usize,
        origin: &'static str,
    }, 
    
    UnknownVersion(u8),

    UnknownPayloadType(u8),

    UnknownExtFormat(u16),

    InvalidPaddingLength(u8),    
}
