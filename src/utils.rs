use iced::Size;

pub trait BoolToggleExt {
    fn toggle(&mut self);
}

impl BoolToggleExt for bool {
    fn toggle(&mut self) {
        *self = !*self;
    }
}

pub trait SizeRatioExt<T> {
    fn get_aspect_ratio(&self) -> T;
}

impl<T> SizeRatioExt<T> for Size<T>
where
    T: std::ops::Div<Output = T> + Copy,
{
    fn get_aspect_ratio(&self) -> T {
        self.width / self.height
    }
}

use std::hash::{DefaultHasher, Hash, Hasher};
/// takes the hash of a single slice and returns it
///
/// convinience function to avoid manually creating a Hasher,
/// only to discard it after using it once.
#[inline]
pub fn hash_chunk<T: Hash>(t: &[T]) -> u64 {
    let mut s = DefaultHasher::new();
    Hash::hash_slice(t, &mut s);
    s.finish()
}
