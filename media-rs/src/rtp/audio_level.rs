use super::error::RtpError;

#[derive(Debug, Clone, Copy)]
pub struct AudioLevelValue {
    pub voice: bool,
    pub volume: AudioLevelVolume,
}

impl Default for AudioLevelValue {
    fn default() -> Self {
        Self { 
            voice: false, 
            volume: AudioLevelVolume(127),
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
            volume: AudioLevelVolume(volume & 0b0111_1111),
        })
    }

    #[inline]
    pub fn to_bytes(&self) -> [u8; 1] {
        
        let b = if self.voice {
            self.volume.0 | 0b1000_0000
        } else {
            self.volume.0
        };

        [ b ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioLevelVolume(pub u8);

impl AudioLevelVolume {
    pub const INF_MIN: Self = Self(128);
}

impl PartialOrd for AudioLevelVolume {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        assert!(AudioLevelVolume(0) == AudioLevelVolume(0));
        assert!(AudioLevelVolume(0) > AudioLevelVolume(127));
        assert!(AudioLevelVolume(127) > AudioLevelVolume::INF_MIN);
    }
}
