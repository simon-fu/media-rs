use super::error::RtpError;

#[derive(Debug, Clone, Copy)]
pub struct AudioLevelValue {
    pub voice: bool,
    pub volume: u8,
}

impl Default for AudioLevelValue {
    fn default() -> Self {
        Self { 
            voice: false, 
            volume: 127,
        }
    }
}

impl AudioLevelValue {
    pub fn parse(data: &[u8]) -> Result<Self, RtpError> {
        if data.len() < 1 {
            return Err(RtpError::NotEnoughBuffer {
                expect: 1,
                actual: data.len(),
                origin: "Audio level value length",
            });
        }

        let volume =data[0];
        Ok(Self {
            voice: (volume & 0b1000_0000) != 0,
            volume: volume & 0b0111_1111,
        })
    }

    #[inline]
    pub fn to_bytes(&self) -> [u8; 1] {
        
        let b = if self.voice {
            self.volume | 0b1000_0000
        } else {
            self.volume
        };

        [ b ]
    }
}
