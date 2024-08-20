use std::pin::Pin;

use futures::Stream;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Stream for Either<L, R>
where
    L: Stream,
    R: Stream<Item = L::Item>,
{
    type Item = L::Item;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match unsafe { self.as_mut().get_unchecked_mut() } {
            Either::Left(left) => unsafe { Pin::new_unchecked(left) }.poll_next(cx),
            Either::Right(right) => unsafe { Pin::new_unchecked(right) }.poll_next(cx),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Either::Left(left) => left.size_hint(),
            Either::Right(right) => right.size_hint(),
        }
    }
}
