use crate::prelude::*;
use futures_lite::{Stream, StreamExt};
use std::future::{ready, Ready};
use std::pin::pin;

use super::{ConcurrentStream, Consumer};

/// A concurrent for each implementation from a `Stream`
#[pin_project::pin_project]
#[derive(Debug)]
pub struct FromStream<S: Stream> {
    #[pin]
    iter: S,
}

impl<S> ConcurrentStream for FromStream<S>
where
    S: Stream,
{
    type Item = S::Item;
    type Future = Ready<Self::Item>;

    async fn drive<C>(self, mut consumer: C) -> C::Output
    where
        C: Consumer<Self::Item, Self::Future>,
    {
        let mut iter = pin!(self.iter);

        // Concurrently progress the consumer as well as the stream. Whenever
        // there is an item from the stream available, we submit it to the
        // consumer and we wait.
        //
        // NOTE(yosh): we're relying on the fact that `Stream::next` can be
        // dropped and recreated freely. That's also true for
        // `Consumer::progress`; though that is intentional. It should be
        // possible to write a combinator which does not drop the `Stream::next`
        // future repeatedly. However for now we're happy to rely on this
        // property here.
        loop {
            // Drive the stream forward
            let a = async {
                let item = iter.next().await;
                State::Item(item)
            };

            // Drive the consumer forward
            let b = async {
                consumer.progress().await;
                State::Progress
            };

            // If an item is available, submit it to the consumer and wait for
            // it to be ready.
            match (a, b).race().await {
                State::Progress => continue,
                State::Item(Some(item)) => consumer.send(ready(item)).await,
                State::Item(None) => break,
            }
        }

        // We will no longer receive items from the underlying stream, which
        // means we're ready to wait for the consumer to finish up.
        consumer.finish().await
    }
}

enum State<T> {
    Progress,
    Item(T),
}

/// Convert into a concurrent stream
pub trait IntoConcurrentStream {
    /// The type of concurrent stream we're returning.
    type ConcurrentStream: ConcurrentStream;

    /// Convert `self` into a concurrent stream.
    fn co(self) -> Self::ConcurrentStream;
}

impl<S: Stream> IntoConcurrentStream for S {
    type ConcurrentStream = FromStream<S>;

    fn co(self) -> Self::ConcurrentStream {
        FromStream { iter: self }
    }
}
