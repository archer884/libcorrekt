use std::marker::PhantomData;
use std::{mem, slice};

/// Iterator over the elements of a null-terminated array
#[derive(Debug, Clone, Copy)]
pub struct Iter<'a, A: 'a>(*const A, PhantomData<&'a A>);

impl<'a, A: 'a> Iter<'a, A> {
    pub fn new(a: *const A) -> Iter<'a, A> {
        Iter(a, PhantomData)
    }
}

unsafe impl<'a, T: Sync> Send for Iter<'a, T> {}
unsafe impl<'a, T: Sync> Sync for Iter<'a, T> {}

impl<'a, A: 'a> Iterator for Iter<'a, A> {
    type Item = &'a A;
    #[inline]
    fn next(&mut self) -> Option<&'a A> {
        unsafe {
            if is_null(&*self.0) {
                None
            } else {
                let ptr = self.0;
                self.0 = ptr.offset(1);
                Some(&*ptr)
            }
        }
    }
}

#[inline]
fn is_null<A>(a: &A) -> bool {
    unsafe {
        let l = mem::size_of_val(a);
        let p = a as *const A as *const u8;
        slice::from_raw_parts(p, l).iter().all(|&b| 0 == b)
    }
}
