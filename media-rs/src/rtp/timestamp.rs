use crate::define_wrapping_type;


define_wrapping_type!(Timestamp, u32, i32);


#[cfg(test)]
mod test_timestamp {

    use crate::define_wrapping_test;

    use super::*;

    define_wrapping_test!(Timestamp, u32, i32);
}

