#![unstable(
    feature = "core_intrinsics_fallbacks",
    reason = "The fallbacks will never be stable, as they exist only to be called \
              by the fallback MIR, but they're exported so they can be tested on \
              platforms where the fallback MIR isn't actually used",
    issue = "none"
)]
#![allow(missing_docs)]

use crate::panicking::panic_nounwind;

/// Ideally we'd do fallbacks using ordinary trait impls, but that doesn't work
/// for const (yet™) so we're stuck with hacky workarounds.
#[inline]
const fn try_as<T: 'static, F: Copy + 'static>(val: F) -> Option<T> {
    if const { super::type_id::<T>() == super::type_id::<F>() } {
        // SAFETY: just checked it's the same type
        Some(unsafe { super::transmute_unchecked(val) })
    } else {
        None
    }
}

macro_rules! if_the_types_work {
    ($f:ident ( $a:expr )) => {
        if let Some(arg) = try_as($a) {
            if let Some(ret) = try_as($f(arg)) {
                return ret;
            }
        }
    };
}

#[rustc_const_unstable(feature = "core_intrinsics_fallbacks", issue = "none")]
const fn wide_mul_u128(a: u128, b: u128) -> (u128, u128) {
    const fn to_low_high(x: u128) -> [u64; 2] {
        [x as u64, (x >> 64) as u64]
    }
    const fn from_low_high(x: [u64; 2]) -> u128 {
        (x[0] as u128) | ((x[1] as u128) << 64)
    }
    #[rustc_const_unstable(feature = "core_intrinsics_fallbacks", issue = "none")]
    const fn scalar_mul(low_high: [u64; 2], k: u64) -> [u64; 3] {
        let (x, c) = u64::widening_mul(k, low_high[0]);
        let (y, z) = u64::carrying_mul(k, low_high[1], c);
        [x, y, z]
    }
    let a = to_low_high(a);
    let b = to_low_high(b);
    let low = scalar_mul(a, b[0]);
    let high = scalar_mul(a, b[1]);
    let r0 = low[0];
    let (r1, c) = u64::overflowing_add(low[1], high[0]);
    let (r2, c) = u64::carrying_add(low[2], high[1], c);
    let r3 = high[2] + (c as u64);
    (from_low_high([r0, r1]), from_low_high([r2, r3]))
}

#[rustc_const_unstable(feature = "core_intrinsics_fallbacks", issue = "none")]
#[inline]
pub const fn carrying_mul_add<T: Copy + 'static>(a: T, b: T, c: T, d: T) -> (T, T) {
    let args = (a, b, c, d);
    macro_rules! via_wider_type {
        ($narrow:ty => $wide:ty) => {{
            #[inline]
            const fn doit(
                (a, b, c, d): ($narrow, $narrow, $narrow, $narrow),
            ) -> ($narrow, $narrow) {
                let (a, b, c, d) = (a as $wide, b as $wide, c as $wide, d as $wide);
                let full = a * b + c + d;
                (full as $narrow, (full >> <$narrow>::BITS) as $narrow)
            }
            if_the_types_work!(doit(args));
        }};
    }
    via_wider_type!(u8 => u16);
    via_wider_type!(u16 => u32);
    via_wider_type!(u32 => u64);
    via_wider_type!(u64 => u128);

    #[rustc_const_unstable(feature = "core_intrinsics_fallbacks", issue = "none")]
    #[inline]
    const fn for_usize((a, b, c, d): (usize, usize, usize, usize)) -> (usize, usize) {
        #[cfg(target_pointer_width = "16")]
        type T = u16;
        #[cfg(target_pointer_width = "32")]
        type T = u32;
        #[cfg(target_pointer_width = "64")]
        type T = u64;

        let (x, y) = carrying_mul_add(a as T, b as T, c as T, d as T);
        (x as usize, y as usize)
    }
    if_the_types_work!(for_usize(args));

    #[rustc_const_unstable(feature = "core_intrinsics_fallbacks", issue = "none")]
    #[inline]
    const fn carrying_mul_add_u128((a, b, c1, c2): (u128, u128, u128, u128)) -> (u128, u128) {
        let (mut r1, mut r2) = wide_mul_u128(a, b);
        let c;
        (r1, c) = u128::overflowing_add(r1, c1);
        r2 += c as u128;
        let c;
        (r1, c) = u128::overflowing_add(r1, c2);
        r2 += c as u128;
        (r1, r2)
    }
    if_the_types_work!(carrying_mul_add_u128(args));

    panic_nounwind("Not supported for this generic type")
}
