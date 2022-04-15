use crate::cm::tables::STRETCH_TABLE;

// Logistic Functions

/// Returns p = 1/(1 + exp(-d)) (Inverse of stretch)
/// d = (-2047..2047), p = (0..4095)
pub fn squash(d: i32) -> i32 {
    const SQ_T: [i32; 33] = [
    1,2,3,6,10,16,27,45,73,120,194,310,488,747,1101,
    1546,2047,2549,2994,3348,3607,3785,3901,3975,4022,
    4050,4068,4079,4085,4089,4092,4093,4094];
    if d > 2047  { return 4095; }
    if d < -2047 { return 0;    }
    let i_w = d & 127;
    let d = ((d >> 7) + 16) as usize;
    (SQ_T[d] * (128 - i_w) + SQ_T[d+1] * i_w + 64) >> 7
}

/// Returns p = ln(d/(1-d)) (Inverse of squash)
/// d = (0..4095), p = (-2047..2047)
pub fn stretch(d: i32) -> i32 {
    assert!(d < 4096);
    STRETCH_TABLE[d as usize] as i32
}


/* Code for generating stretch table
pub struct Stretch {
    stretch_table: [i16; 4096],
}
impl Stretch {
    pub fn new() -> Stretch {
        let mut st = Stretch {
            stretch_table: [0; 4096],
        };
        let mut pi = 0;
        for x in -2047..=2047 {
            let i = squash(x);
            for j in pi..=i {
                st.stretch_table[j as usize] = x as i16;
            }
            pi = i + 1;
        }
        st.stretch_table[4095] = 2047;
        st
    }
}
*/
