// LNP/BP Core Library implementing LNPBP specifications & standards
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

//! This is a trick for rust compiler helping to distinguish types implementing
//! mutually-exclusive traits (required until negative trait impls will be there)
//! Implemented after concept by Martin Habovštiak <martin.habovstiak@gmail.com>

use core::marker::PhantomData;

pub struct Holder<T, S>(T, PhantomData<S>);
impl<T, S> Holder<T, S> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self(val, PhantomData::<S>::default())
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }

    #[inline]
    pub fn as_inner(&self) -> &T {
        &self.0
    }
}
