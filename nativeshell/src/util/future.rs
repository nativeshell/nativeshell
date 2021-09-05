use std::{cell::RefCell, rc::Rc, task::Poll};

use futures::Future;

//
// Convert callback-oriented code to futures
//

#[derive(Clone)]
pub struct FutureFulfillment<T> {
    waker: std::task::Waker,
    data: Rc<RefCell<Option<T>>>,
}

impl<T> FutureFulfillment<T> {
    pub fn fulfill(&self, result: T) {
        {
            let mut data = self.data.borrow_mut();
            data.replace(result);
        }
        let waker = self.waker.clone();
        waker.wake();
    }
}

pub struct FutureWrapper<T, F>
where
    F: FnOnce(FutureFulfillment<T>) + 'static,
{
    data: Rc<RefCell<Option<T>>>,
    callback: RefCell<Option<F>>,
    in_progress: RefCell<bool>,
}

impl<T, F> Future for FutureWrapper<T, F>
where
    F: FnOnce(FutureFulfillment<T>) + 'static,
{
    type Output = T;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let was_in_progress = self.in_progress.replace(true);
        let res: Option<T>;
        {
            let mut mutable = self.data.borrow_mut();
            res = mutable.take();
        }
        match res {
            Some(data) => Poll::Ready(data),
            None => {
                if !was_in_progress {
                    let fulfillment = FutureFulfillment {
                        waker: cx.waker().clone(),
                        data: self.data.clone(),
                    };
                    (self.callback.take().unwrap())(fulfillment);
                }
                std::task::Poll::Pending
            }
        }
    }
}

impl<T, F> FutureWrapper<T, F>
where
    F: FnOnce(FutureFulfillment<T>) + 'static,
{
    pub fn create(function: F) -> impl Future<Output = T> {
        FutureWrapper {
            data: Rc::new(RefCell::new(None)),
            callback: RefCell::new(Some(function)),
            in_progress: RefCell::new(false),
        }
    }
}
