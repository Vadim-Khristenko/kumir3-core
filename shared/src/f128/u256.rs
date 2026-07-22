//! Wide-integer helpers backing [`F128`] multiplication, division and
//! Payne-Hanek argument reduction.

use std::cmp::Ordering;

// =============================================================================
//         SECTION: INTERNALS (U256)
// =============================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct U256 {
    pub(super) d: [u64; 4],
}

impl U256 {
    const ZERO: Self = Self { d: [0, 0, 0, 0] };

    pub(super) fn from_u128(x: u128) -> Self {
        Self {
            d: [x as u64, (x >> 64) as u64, 0, 0],
        }
    }

    pub(super) fn mul_u128(a: u128, b: u128) -> Self {
        let a_lo = a & 0xFFFFFFFFFFFFFFFFu128;
        let a_hi = a >> 64;
        let b_lo = b & 0xFFFFFFFFFFFFFFFFu128;
        let b_hi = b >> 64;

        let p0 = a_lo * b_lo;
        let p1 = a_lo * b_hi;
        let p2 = a_hi * b_lo;
        let p3 = a_hi * b_hi;

        let mask = 0xFFFFFFFFFFFFFFFFu128;
        let r0 = p0 & mask;
        let t1 = (p0 >> 64) + (p1 & mask) + (p2 & mask);
        let r1 = t1 & mask;
        let t2 = (p1 >> 64) + (p2 >> 64) + (p3 & mask) + (t1 >> 64);
        let r2 = t2 & mask;
        let r3 = (p3 >> 64) + (t2 >> 64);

        Self {
            d: [r0 as u64, r1 as u64, r2 as u64, r3 as u64],
        }
    }

    /// Умножение u128 на U256 -> U384 (возвращаем как U256 + carry)
    pub(super) fn mul_u128_u256(a: u128, b: U256) -> U512 {
        let a_lo = a as u64;
        let a_hi = (a >> 64) as u64;
        let mut res = [0u64; 8]; // 512 бит результат

        // a_lo * b (младшая половина a)
        let mut carry = 0u64;
        for (j, res_j) in res.iter_mut().take(4).enumerate() {
            let prod = (a_lo as u128) * (b.d[j] as u128) + (*res_j as u128) + (carry as u128);
            *res_j = prod as u64;
            carry = (prod >> 64) as u64;
        }
        res[4] = carry;

        // a_hi * b (старшая половина a, сдвинутая на 64 бита)
        carry = 0u64;
        for (j, res_k) in res.iter_mut().skip(1).take(4).enumerate() {
            let prod = (a_hi as u128) * (b.d[j] as u128) + (*res_k as u128) + (carry as u128);
            *res_k = prod as u64;
            carry = (prod >> 64) as u64;
        }
        res[5] = carry;

        U512 { d: res }
    }

    pub(super) fn shr(&self, shift: u32) -> Self {
        if shift == 0 || self.is_zero() {
            return *self;
        }
        if shift >= 256 {
            return Self::ZERO;
        }

        let limb_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let mut out = [0u64; 4];

        for i in limb_shift..4 {
            let src = i;
            let dest = i - limb_shift;
            out[dest] = if bit_shift == 0 {
                self.d[src]
            } else {
                (self.d[src] >> bit_shift)
                    | (if src + 1 < 4 {
                        self.d[src + 1] << (64 - bit_shift)
                    } else {
                        0
                    })
            };
        }
        Self { d: out }
    }

    pub(super) fn shl(self, shift: u32) -> Self {
        if shift == 0 || self.is_zero() {
            return self;
        }
        if shift >= 256 {
            return Self::ZERO;
        }

        let limb_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let mut out = [0u64; 4];

        for i in 0..4 {
            let dest = i + limb_shift;
            if dest < 4 {
                out[dest] |= self.d[i] << bit_shift;
                if bit_shift > 0 && dest + 1 < 4 {
                    out[dest + 1] |= self.d[i] >> (64 - bit_shift);
                }
            }
        }
        Self { d: out }
    }

    pub(super) fn sub(self, other: Self) -> Self {
        let mut out = [0u64; 4];
        let mut borrow = false;
        for ((out_i, self_i), other_i) in out.iter_mut().zip(self.d.iter()).zip(other.d.iter()) {
            let (diff, b1) = self_i.overflowing_sub(*other_i);
            let (res, b2) = diff.overflowing_sub(borrow as u64);
            *out_i = res;
            borrow = b1 || b2;
        }
        debug_assert!(!borrow, "U256 underflow");
        Self { d: out }
    }

    pub(super) fn cmp(&self, other: &Self) -> Ordering {
        for i in (0..4).rev() {
            match self.d[i].cmp(&other.d[i]) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        Ordering::Equal
    }

    pub(super) fn is_zero(&self) -> bool {
        self.d.iter().all(|&x| x == 0)
    }

    pub(super) fn leading_zeros(&self) -> u32 {
        for i in (0..4).rev() {
            if self.d[i] != 0 {
                return self.d[i].leading_zeros() + (3 - i) as u32 * 64;
            }
        }
        256
    }

    pub(super) fn low_u128(&self) -> u128 {
        ((self.d[1] as u128) << 64) | (self.d[0] as u128)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct U512 {
    d: [u64; 8],
}

impl U512 {
    pub(super) fn shr(&self, shift: u32) -> U256 {
        if shift >= 512 {
            return U256::ZERO;
        }
        let limb_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let mut out = [0u64; 4];

        for i in limb_shift..8 {
            if i - limb_shift >= 4 {
                break;
            }
            let dest = i - limb_shift;
            out[dest] = if bit_shift == 0 {
                self.d[i]
            } else {
                (self.d[i] >> bit_shift)
                    | (if i + 1 < 8 {
                        self.d[i + 1] << (64 - bit_shift)
                    } else {
                        0
                    })
            };
        }
        U256 { d: out }
    }

    pub(super) fn low_u128(&self) -> u128 {
        ((self.d[1] as u128) << 64) | (self.d[0] as u128)
    }
}
