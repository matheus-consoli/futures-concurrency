use super::FirstOk as FirstOkTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

#[async_trait::async_trait(?Send)]
impl<F, T, E, const N: usize> FirstOkTrait for [F; N]
where
    T: std::fmt::Debug,
    F: Future<Output = Result<T, E>>,
    E: fmt::Debug,
{
    type Output = T;
    type Error = E;

    async fn first_ok(self) -> Result<Self::Output, Self::Error> {
        FirstOk {
            elems: self.map(MaybeDone::new),
        }
        .await
    }
}

/// Waits for two similarly-typed futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the
/// futures once both complete.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct FirstOk<F, T, E, const N: usize>
where
    T: fmt::Debug,
    F: Future<Output = Result<T, E>>,
{
    elems: [MaybeDone<F>; N],
}

impl<F, T, E, const N: usize> fmt::Debug for FirstOk<F, T, E, N>
where
    F: Future<Output = Result<T, E>> + fmt::Debug,
    F::Output: fmt::Debug,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Join").field("elems", &self.elems).finish()
    }
}

impl<F, T, E, const N: usize> Future for FirstOk<F, T, E, N>
where
    T: fmt::Debug,
    F: Future<Output = Result<T, E>>,
    E: fmt::Debug,
{
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        let this = self.project();

        for elem in this.elems.iter_mut() {
            // SAFETY: we don't ever move the pinned container here; we only pin project
            let mut elem = unsafe { Pin::new_unchecked(elem) };
            if let Poll::Pending = elem.as_mut().poll(cx) {
                all_done = false
            } else if let Some(Err(_)) = elem.as_ref().output() {
                return Poll::Ready(Err(elem.take().unwrap().unwrap_err()));
            }
        }

        if all_done {
            use core::mem::MaybeUninit;

            // Create the result array based on the indices
            let mut out: [MaybeUninit<T>; N] = {
                // inlined version of unstable `MaybeUninit::uninit_array()`
                // TODO: replace with `MaybeUninit::uninit_array()` when it becomes stable
                unsafe { MaybeUninit::<[MaybeUninit<_>; N]>::uninit().assume_init() }
            };

            // NOTE: this clippy attribute can be removed once we can `collect` into `[usize; K]`.
            #[allow(clippy::clippy::needless_range_loop)]
            for (i, el) in this.elems.iter_mut().enumerate() {
                // SAFETY: we don't ever move the pinned container here; we only pin project
                let el = unsafe { Pin::new_unchecked(el) }.take().unwrap().unwrap();
                out[i] = MaybeUninit::new(el);
            }
            let result = unsafe { out.as_ptr().cast::<[T; N]>().read() };
            Poll::Ready(Ok(result))
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future;
    use std::io::{self, Error, ErrorKind};

    #[test]
    fn all_ok() {
        async_io::block_on(async {
            let res: io::Result<_> = [future::ready(Ok("hello")), future::ready(Ok("world"))]
                .first_ok()
                .await;
            assert_eq!(res.unwrap(), ["hello", "world"]);
        })
    }

    #[test]
    fn one_err() {
        async_io::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: io::Result<_> = [future::ready(Ok("hello")), future::ready(Err(err))]
                .first_ok()
                .await;
            assert_eq!(res.unwrap_err().to_string(), String::from("oh no"));
        });
    }
}
