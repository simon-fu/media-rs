
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Seq(pub(super) u16);
impl Seq {
    
    pub fn next(self) -> Seq {
        Seq(self.0.wrapping_add(1))
    }

    pub fn precedes(self, other: Seq) -> bool {
        self.next() == other
    }
}
impl From<Seq> for u16 {
    fn from(v: Seq) -> Self {
        v.0
    }
}
impl From<u16> for Seq {
    fn from(v: u16) -> Self {
        Seq(v)
    }
}

impl std::ops::Sub for Seq {
    type Output = i32;

    fn sub(self, rhs: Seq) -> Self::Output {
        let delta = i32::from(self.0) - i32::from(rhs.0);
        if delta < std::i16::MIN as i32 {
            std::u16::MAX as i32 + 1 + delta
        } else if delta > std::i16::MAX as i32 {
            delta - std::u16::MAX as i32 - 1
        } else {
            delta
        }
    }
}
impl PartialOrd for Seq {
    fn partial_cmp(&self, other: &Seq) -> Option<std::cmp::Ordering> {
        (*self - *other).partial_cmp(&0)
    }
}
impl Ord for Seq {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self - *other).cmp(&0)
    }
}

impl std::ops::Add<u16> for Seq {
    type Output = Seq;

    fn add(self, rhs: u16) -> Self::Output {
        Seq(self.0.wrapping_add(rhs))
    }
}

pub trait IntoSeqIterator {
    fn seq_iter(self) -> SeqIter;
}
impl IntoSeqIterator for std::ops::Range<Seq> {
    fn seq_iter(self) -> SeqIter {
        SeqIter(self.start, self.end)
    }
}


/// Usage:  
/// ```
/// for seq in (50..60).seq_iter() {
///     println!("{:?}", seq);
/// }
/// ```
pub struct SeqIter(Seq, Seq);
impl Iterator for SeqIter {
    type Item = Seq;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 >= self.1 {
            None
        } else {
            let res = self.0;
            self.0 = self.0.next();
            Some(res)
        }
    }
}

