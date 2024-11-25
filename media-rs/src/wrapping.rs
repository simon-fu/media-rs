
#[macro_export]
macro_rules! define_wrapping_type {
    ($name:ident, $unsigned_type:ident, $signed_type:ident) => {
        #[derive(PartialEq, Eq, Debug, Clone, Copy, Default)]
        pub struct $name(pub $unsigned_type);
        impl $name {
            const SIGNED_ZERO: $signed_type = 0;

            pub fn next(self) -> Self {
                Self(self.0.wrapping_add(1))
            }
        
            pub fn precedes(self, other: Self) -> bool {
                self.next() == other
            }
        }
        impl From<$name> for $unsigned_type {
            fn from(v: $name) -> Self {
                v.0
            }
        }
        impl From<$unsigned_type> for $name {
            fn from(v: $unsigned_type) -> Self {
                Self(v)
            }
        }

        impl std::ops::Sub for $name {
            type Output = $signed_type;
        
            fn sub(self, rhs: Self) -> Self::Output {
                // Calc distance
                // The max distance is $signed_type::MAX.abs();
                let d1 = self.0.wrapping_sub(rhs.0);
                let d2 = rhs.0.wrapping_sub(self.0);

                // get min distance
                if d1 < d2 {
                    d1 as $signed_type
                } else {
                    $name::SIGNED_ZERO.wrapping_sub(d2 as $signed_type) 
                }
            }
        }
        
        impl std::ops::Add<$signed_type> for $name {
            type Output = $name;
        
            fn add(self, rhs: $signed_type) -> Self::Output {
                Self(self.0.wrapping_add_signed(rhs))
            }
        }

        impl std::ops::AddAssign<$signed_type> for $name {
            fn add_assign(&mut self, rhs: $signed_type) {
                *self = (*self) + rhs;
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                (*self - *other).partial_cmp(&0)
            }
        }
        
        impl Ord for $name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                (*self - *other).cmp(&0)
            }
        }
        
        impl std::ops::Add<$unsigned_type> for $name {
            type Output = $name;
        
            fn add(self, rhs: $unsigned_type) -> Self::Output {
                Self(self.0.wrapping_add(rhs))
            }
        }

        ::paste::paste! {
    
            pub struct [<$name Iter>](pub $name, pub $name);
            impl Iterator for [<$name Iter>] {
                type Item = $name;

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
        }

    };
}

#[macro_export]
macro_rules! define_wrapping_test {
    ($name:ident, $unsigned_type:ident, $signed_type:ident) => {
        
        #[test]
        fn test() {
            const UMAX: $unsigned_type = $unsigned_type::MAX;
            const IMAX: $signed_type = $signed_type::MAX;
            const IMIN: $signed_type = $signed_type::MIN;
            const UIMAX: $unsigned_type = IMAX as $unsigned_type;

            check_delta(10, 9, 1, -1);
            
            check_delta(1, 0, 1, -1);
            
            check_delta(0, UMAX, 1, -1);
            
            check_delta(UMAX, UMAX-UIMAX+1, IMAX-1, -(IMAX-1));

            check_delta(UMAX, UMAX-UIMAX-0, IMAX-0, -(IMAX-0));
            
            check_delta(UMAX, UMAX-UIMAX-1, IMIN, IMIN);
            check_delta(UMAX, UMAX-UIMAX-2, -(IMAX-0), IMAX-0);
            check_delta(UMAX, UMAX-UIMAX-3, -(IMAX-1), IMAX-1);
        }
    
        fn check_delta(
            next: $unsigned_type, 
            current: $unsigned_type, 
            delta: $signed_type, 
            rdelta: $signed_type,
        ) {
            // let rdelta = 0.wrapping_sub(delta);
    
            assert_eq!($name::from(next)-$name::from(current), delta);
            assert_eq!($name::from(current)-$name::from(next), rdelta);
    
            assert_eq!($name::from(next) + rdelta, $name::from(current));
            assert_eq!($name::from(current) + delta, $name::from(next));
            
            assert_eq!($name::from(next)-$name::from(next), 0);
            assert_eq!($name::from(current)-$name::from(current), 0);
        }
    };
}



