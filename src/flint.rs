use rug::Integer;
use std::ffi::{c_void, CStr};
use std::os::raw::{c_char, c_int, c_long};

#[repr(C)]
struct MpzRaw {
    alloc: c_int,
    size: c_int,
    limbs: *mut flint3_sys::ulong,
}

unsafe extern "C" {
    fn fmpz_set_mpz(f: *mut flint3_sys::fmpz, x: *const c_void);
    fn fmpz_get_mpz(x: *mut c_void, f: *const flint3_sys::fmpz);
    fn flint_mpn_get_str(
        res: *mut c_char,
        base: c_int,
        x: *const flint3_sys::ulong,
        xn: c_long,
        negative: c_int,
    ) -> *mut c_char;
    fn _flint_mpn_mul(
        r: *mut flint3_sys::ulong,
        x: *const flint3_sys::ulong,
        xn: c_long,
        y: *const flint3_sys::ulong,
        yn: c_long,
    ) -> flint3_sys::ulong;
    fn _flint_mpn_sqr(
        r: *mut flint3_sys::ulong,
        x: *const flint3_sys::ulong,
        n: c_long,
    ) -> flint3_sys::ulong;
}

struct Fmpz(flint3_sys::fmpz_t);

impl Fmpz {
    fn new() -> Self {
        let mut inner: flint3_sys::fmpz_t = [0];
        unsafe { flint3_sys::fmpz_init(inner.as_mut_ptr()) };
        Self(inner)
    }

    fn from_integer(value: &Integer) -> Self {
        let mut out = Self::new();
        unsafe { fmpz_set_mpz(out.0.as_mut_ptr(), value.as_raw().cast()) };
        out
    }

    fn to_integer(&self) -> Integer {
        let mut out = Integer::new();
        unsafe { fmpz_get_mpz(out.as_raw_mut().cast(), self.0.as_ptr()) };
        out
    }
}

impl Drop for Fmpz {
    fn drop(&mut self) {
        unsafe { flint3_sys::fmpz_clear(self.0.as_mut_ptr()) };
    }
}

pub fn set_workers(workers: usize) {
    unsafe { flint3_sys::flint_set_num_threads(workers as i32) };
}

fn flint_mul(left: &Integer, right: &Integer) -> Integer {
    let (left, right) = if left.significant_bits() >= right.significant_bits() {
        (left, right)
    } else {
        (right, left)
    };
    let left_raw = unsafe { &*left.as_raw().cast::<MpzRaw>() };
    let right_raw = unsafe { &*right.as_raw().cast::<MpzRaw>() };
    let xn = left_raw.size.unsigned_abs() as usize;
    let yn = right_raw.size.unsigned_abs() as usize;
    if xn == 0 || yn == 0 {
        return Integer::new();
    }

    let mut out = Integer::with_capacity((xn + yn) * 64);
    let out_raw = unsafe { &mut *out.as_raw_mut().cast::<MpzRaw>() };
    let same = std::ptr::eq(left, right);
    let top = unsafe {
        if same && xn == yn {
            _flint_mpn_sqr(out_raw.limbs, left_raw.limbs, xn as c_long)
        } else {
            _flint_mpn_mul(
                out_raw.limbs,
                left_raw.limbs,
                xn as c_long,
                right_raw.limbs,
                yn as c_long,
            )
        }
    };
    let mut zn = xn + yn - usize::from(top == 0);
    while zn > 1 && unsafe { *out_raw.limbs.add(zn - 1) } == 0 {
        zn -= 1;
    }
    let negative = (left_raw.size < 0) != (right_raw.size < 0);
    out_raw.size = if negative {
        -(zn as c_int)
    } else {
        zn as c_int
    };
    out
}

pub fn multiply(left: &Integer, right: &Integer) -> Integer {
    if left.significant_bits().max(right.significant_bits()) >= 50_000 {
        flint_mul(left, right)
    } else {
        Integer::from(left * right)
    }
}

pub fn divide(left: &Integer, right: &Integer) -> Integer {
    if left.significant_bits() < 50_000 {
        return Integer::from(left / right);
    }
    let numerator = Fmpz::from_integer(left);
    let denominator = Fmpz::from_integer(right);
    let mut quotient = Fmpz::new();
    unsafe {
        flint3_sys::fmpz_tdiv_q(
            quotient.0.as_mut_ptr(),
            numerator.0.as_ptr(),
            denominator.0.as_ptr(),
        )
    };
    quotient.to_integer()
}

pub fn decimal_string(value: &Integer) -> String {
    let raw = unsafe { &*value.as_raw().cast::<MpzRaw>() };
    let limbs = raw.size.unsigned_abs() as usize;
    if limbs < 1_400 {
        return value.to_string();
    }
    unsafe {
        let text = flint_mpn_get_str(
            std::ptr::null_mut(),
            10,
            raw.limbs,
            limbs as c_long,
            c_int::from(raw.size < 0),
        );
        let out = CStr::from_ptr(text).to_string_lossy().into_owned();
        flint3_sys::flint_free(text.cast());
        out
    }
}
