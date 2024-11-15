
use crate::define_wrapping_type;

define_wrapping_type!(Seq, u16, i16);



#[cfg(test)]
mod test_seq {

    use crate::define_wrapping_test;

    use super::*;

    define_wrapping_test!(Seq, u16, i16);
}




// #[derive(PartialEq, Eq, Debug, Clone, Copy)]
// pub struct Seq(pub(super) u16);
// impl Seq {
    
//     pub fn next(self) -> Seq {
//         Seq(self.0.wrapping_add(1))
//     }

//     pub fn precedes(self, other: Seq) -> bool {
//         self.next() == other
//     }
// }
// impl From<Seq> for u16 {
//     fn from(v: Seq) -> Self {
//         v.0
//     }
// }
// impl From<u16> for Seq {
//     fn from(v: u16) -> Self {
//         Seq(v)
//     }
// }

// // impl std::ops::Sub for Seq {
// //     type Output = i32;

// //     fn sub(self, rhs: Seq) -> Self::Output {
// //         let delta = i32::from(self.0) - i32::from(rhs.0);
// //         if delta < std::i16::MIN as i32 {
// //             std::u16::MAX as i32 + 1 + delta
// //         } else if delta > std::i16::MAX as i32 {
// //             delta - std::u16::MAX as i32 - 1
// //         } else {
// //             delta
// //         }
// //     }
// // }

// // impl std::ops::Add<i32> for Seq {
// //     type Output = Seq;

// //     fn add(self, rhs: i32) -> Self::Output {
// //         if rhs > 0 {
// //             let delta = (rhs & 0xFFFF) as u16;
// //             Seq(self.0.wrapping_add(delta))
// //         } else {
// //             let delta = (-rhs & 0xFFFF) as u16;
// //             Seq(self.0.wrapping_sub(delta))
// //         }
// //     }
// // }

// impl std::ops::Sub for Seq {
//     type Output = i16;

//     fn sub(self, rhs: Seq) -> Self::Output {
//         seq_delta(self.0, rhs.0)
//     }
// }

// impl std::ops::Add<i16> for Seq {
//     type Output = Seq;

//     fn add(self, rhs: i16) -> Self::Output {
//         Self(self.0.wrapping_add_signed(rhs))
//     }
// }

// #[inline]
// fn seq_delta(next: u16, current: u16) -> i16 {
//     // Calc distance
//     // The max distance is i16::MAX.abs();
//     let d1 = next.wrapping_sub(current);
//     let d2 = current.wrapping_sub(next);

//     // get min distance
//     if d1 < d2 {
//         d1 as i16
//     } else {
//         0_i16.wrapping_sub(d2 as i16) 
//     }
// }

// // #[inline]
// // fn seq_delta(next: u16, current: u16) -> i16 {
// //     let delta = i32::from(next) - i32::from(current);
// //     let r = if delta < std::i16::MIN as i32 {
// //         std::u16::MAX as i32 + 1 + delta
// //     } else if delta > std::i16::MAX as i32 {
// //         delta - std::u16::MAX as i32 - 1
// //     } else {
// //         delta
// //     };
// //     r as i16
// // }

// // /// from chatgpt  
// // #[inline]
// // fn seq_delta(next: u16, current: u16) -> i16 {
// //     // let diff = next as i32 - current as i32;
// //     let diff = i32::from(next) - i32::from(current);
// //     ((diff + 32768) % 65536 - 32768) as i16
// // }

// impl PartialOrd for Seq {
//     fn partial_cmp(&self, other: &Seq) -> Option<std::cmp::Ordering> {
//         (*self - *other).partial_cmp(&0)
//     }
// }

// impl Ord for Seq {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         (*self - *other).cmp(&0)
//     }
// }

// impl std::ops::Add<u16> for Seq {
//     type Output = Seq;

//     fn add(self, rhs: u16) -> Self::Output {
//         Seq(self.0.wrapping_add(rhs))
//     }
// }



// pub trait IntoSeqIterator {
//     fn seq_iter(self) -> SeqIter;
// }
// impl IntoSeqIterator for std::ops::Range<Seq> {
//     fn seq_iter(self) -> SeqIter {
//         SeqIter(self.start, self.end)
//     }
// }


// /// Usage:  
// /// ```
// /// for seq in (50..60).seq_iter() {
// ///     println!("{:?}", seq);
// /// }
// /// ```
// pub struct SeqIter(Seq, Seq);
// impl Iterator for SeqIter {
//     type Item = Seq;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.0 >= self.1 {
//             None
//         } else {
//             let res = self.0;
//             self.0 = self.0.next();
//             Some(res)
//         }
//     }
// }



// #[cfg(test)]
// mod test {

//     use super::*;


//     #[test]
//     fn test() {

//         check_delta(10, 9, 1, -1);
        
//         check_delta(1, 0, 1, -1);
        
//         check_delta(0, u16::MAX, 1, -1);
        
//         check_delta(u16::MAX, u16::MAX-(i16::MAX as u16)+1, i16::MAX-1, -(i16::MAX-1));

//         check_delta(u16::MAX, u16::MAX-(i16::MAX as u16)-0, i16::MAX, -i16::MAX);

//         check_delta(u16::MAX, u16::MAX-(i16::MAX as u16)-1, i16::MIN, i16::MIN);

//         check_delta(u16::MAX, u16::MAX-(i16::MAX as u16)-2, -i16::MAX, i16::MAX);

//         check_delta(u16::MAX, u16::MAX-(i16::MAX as u16)-3, -(i16::MAX-1), i16::MAX-1);
//     }

//     fn check_delta(next: u16, current: u16, delta: i16, rdelta: i16) {
//         // let rdelta = 0.wrapping_sub(delta);

//         assert_eq!(Seq::from(next)-Seq::from(current), delta);
//         assert_eq!(Seq::from(current)-Seq::from(next), rdelta);

//         assert_eq!(Seq::from(next) + rdelta, Seq::from(current));
//         assert_eq!(Seq::from(current) + delta, Seq::from(next));
        
//         assert_eq!(Seq::from(next)-Seq::from(next), 0);
//         assert_eq!(Seq::from(current)-Seq::from(current), 0);
//     }

// }

