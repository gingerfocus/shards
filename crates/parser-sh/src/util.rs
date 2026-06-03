#![allow(unused)]

use std::{
    fmt::Debug,
    mem,
    sync::{atomic::AtomicPtr, Arc},
};

/// An iterator over an owned set of chars.
///
/// # Safty
/// I believe this struct to be safe because the String is
/// heap-allocated (stable address) and will never be modified
/// (stable address). `chars` will not outlive the struct, so
/// lying about the lifetime should be fine.
///
/// `Chars` doesn't have a destructor so it is safe to Drop.
///
/// source: https://stackoverflow.com/a/47207490
pub(crate) struct OwnedCharBuffer {
    _s: String,
    /// the chars here are borrowed from the above string
    chars: std::str::Chars<'static>,
}

impl From<String> for OwnedCharBuffer {
    fn from(value: String) -> Self {
        // safety: chars will not escape the scope of this object
        let chars = unsafe { mem::transmute(value.chars()) };
        Self { _s: value, chars }
    }
}

impl OwnedCharBuffer {
    pub fn new(value: String) -> Self {
        Self::from(value)
    }
}

impl Iterator for OwnedCharBuffer {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.chars.next()
    }
}

/// An alternative to a hash map which is backed by a static array. Good for
/// small inputs.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct StaticMap<K, V>(Vec<(K, V)>);

impl<K, V> StaticMap<K, V>
where
    K: PartialEq,
{
    pub const fn new() -> Self {
        StaticMap(Vec::new())
    }

    pub fn insert(&mut self, key: K, mut value: V) -> Option<V> {
        if let Some(v) = self.get_mut(&key) {
            std::mem::swap(v, &mut value);
            Some(value)
        } else {
            self.0.push((key, value));
            None
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.0.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn contains(&self, key: &K) -> bool {
        self.0.iter().any(|(k, _)| k == key)
    }
}

impl<K, V> IntoIterator for StaticMap<K, V> {
    type Item = (K, V);

    type IntoIter = std::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// An Atomic Buffer is a growable data structure that can give give read only
/// acsess to data it has already written.
///
/// You can only push to it as removing data might cause veiws into it to
/// become invalid.
pub(crate) struct AtomicBuffer<T> {
    ptr: Arc<AtomicPtr<T>>,
    len: usize,
    cap: usize,
}

impl<T: Debug> Debug for AtomicBuffer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::sync::atomic::Ordering;
        let mut d = f.debug_list();

        let _ = self
            .ptr
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |ptr| {
                // todo
                for i in 0..=self.len {
                    d.entry(&unsafe { ptr.add(i).read() });
                }

                None
            });
        d.finish()
    }
}

impl<T: Clone> AtomicBuffer<T> {
    pub fn new() -> Self {
        let inner = Vec::new();
        let (ptr, len, cap) = inner.into_raw_parts();
        let ptr = AtomicPtr::new(ptr);
        Self {
            ptr: Arc::new(ptr),
            len,
            cap,
        }
    }

    pub fn push(&mut self, value: T) {
        use std::sync::atomic::Ordering;
        let _ = self
            .ptr
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |ptr| {
                let mut v = unsafe { Vec::from_raw_parts(ptr, self.len, self.cap) };
                v.push(value.clone());
                let (new_ptr, len, cap) = v.into_raw_parts();
                println!("old: {:?}, new: {:?}", ptr, new_ptr);
                self.len = len;
                self.cap = cap;
                Some(new_ptr)
            });
    }

    pub fn get_slice(&self, str: usize, end: usize) -> AtomicSlice<T> {
        AtomicSlice {
            ptr: self.ptr.clone(),
            str,
            end,
        }
    }
}

pub(crate) struct AtomicSlice<T> {
    ptr: Arc<AtomicPtr<T>>,
    str: usize,
    end: usize,
}

impl<T: Debug> Debug for AtomicSlice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::sync::atomic::Ordering;
        let mut d = f.debug_list();

        let _ = self
            .ptr
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |ptr| {
                for i in self.str..=self.end {
                    d.entry(&unsafe { ptr.add(i).read() });
                }
                None
            });
        d.finish()
    }
}

impl<T> AtomicSlice<T> {
    /// Copies the data into a vec
    pub fn to_vec(&self) -> Vec<T> {
        use std::sync::atomic::Ordering;

        let mut v = Vec::with_capacity(self.end - self.str);

        let _ = self
            .ptr
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |ptr| {
                eprintln!("reading from2: {:?}", ptr);
                // todo
                for i in self.str..=self.end {
                    v.push(unsafe { ptr.add(i).read() });
                }

                None
            });
        v
    }

    /// The data cannot be acsess directly so it is required that functions
    /// be passed in here so they can be run atomically.
    ///
    /// See [`AtomicPtr::fetch_update`] for more details.
    pub fn with_slice<F>(&self, mut f: F)
    where
        F: FnMut(&[T]),
    {
        use std::sync::atomic::Ordering;

        let _ = self
            .ptr
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |ptr| {
                let slc =
                    unsafe { std::slice::from_raw_parts(ptr.add(self.str), self.end - self.str) };
                f(slc);

                None
            });
    }
}

#[cfg(test)]
mod test {
    use super::AtomicBuffer;

    #[test]
    fn atomic_ptr_follow() {
        let mut buf = AtomicBuffer::new();

        buf.push(3);
        buf.push(2);
        buf.push(3);
        buf.push(7);
        buf.push(5);
        let s1 = buf.get_slice(1, 2); // 2, 3
        let s2 = buf.get_slice(2, 4); // 3, 7, 5
        assert_eq!(s1.to_vec(), vec![2, 3]);
        assert_eq!(s2.to_vec(), vec![3, 7, 5]);
        buf.push(5);
        buf.push(8);
        buf.push(5); //  <-- reallocation
        buf.push(8);
        assert_eq!(s1.to_vec(), vec![2, 3]);
        assert_eq!(s2.to_vec(), vec![3, 7, 5]);

        for i in 0..16 {
            buf.push(i); //  <-- reallocation in here
        }

        assert_eq!(s1.to_vec(), vec![2, 3]);
        assert_eq!(s2.to_vec(), vec![3, 7, 5]);
    }
}
