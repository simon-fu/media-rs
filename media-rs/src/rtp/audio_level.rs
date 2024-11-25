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
            volume: AudioLevelVolume::MIN,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord)]
pub struct AudioLevelVolume(pub u8);

impl AudioLevelVolume {
    pub const MAX: Self = Self(0);
    pub const MIN: Self = Self(127);
    pub const INF_MIN: Self = Self(128);
    const MASK: u8 = 0x7F;

    pub fn from_i64(val: i64) -> Self {
        Self((val as u8) & Self::MASK)
    }

    pub fn as_i64(&self) -> i64 {
        self.0 as i64
    }
}

impl PartialOrd for AudioLevelVolume {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl std::ops::Add<u8> for AudioLevelVolume {
    type Output = AudioLevelVolume;

    fn add(self, rhs: u8) -> Self::Output {
        if self.0 >= rhs {
            Self(self.0 - rhs)
        } else {
            Self::MAX
        }
    }
}

impl std::ops::Sub<u8> for AudioLevelVolume {
    type Output = AudioLevelVolume;

    fn sub(self, rhs: u8) -> Self::Output {
        let v = self.0.saturating_add(rhs);
        if v <= Self::MIN.0 {
            Self(v)
        } else {
            Self::MIN
        }
    }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        assert!(AudioLevelVolume(0) == AudioLevelVolume(0));
        assert!(AudioLevelVolume(0) > AudioLevelVolume::MIN);
        assert!(AudioLevelVolume::MIN > AudioLevelVolume::INF_MIN);
    }
}
