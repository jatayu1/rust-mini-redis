use tokio::net::TcpStream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::{Future, self};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use futures::task::{self, ArcWake};
use std::thread;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};


async fn my_async_fn() {
    println!("hello from async");

    let _socket = TcpStream::connect("127.0.0.1:3000").await.unwrap();

    println!("async TCP operation complete");
}

// pub trait Future {
//     type Output;
//     fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>;
// }

struct Delay {
    when: Instant,
}

impl Future for Delay {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.when {
            println!("Hello world");
            Poll::Ready("done")
        } else {
            // Ignore
            // cx.waker().wake_by_ref();

            // Get a handle to the waker for the current task
            let waker = cx.waker().clone();
            let when = self.when;

            // Spawn a timer thread.
            thread::spawn(move || {
                let now = Instant::now();

                if now < when {
                    thread::sleep(when - now);
                }

                waker.wake();
            });
            Poll::Pending
        }
    }
}

enum MainFuture {
    State0,
    State1(Delay),
    Terminated,
}

impl Future for MainFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use MainFuture::*;

        loop {
            match *self {
                State0 => {
                    let when = Instant::now() + Duration::from_millis(10);
                    let future = Delay { when };
                    *self = State1(future);
                }
                State1(ref mut my_future) => {
                    match Pin::new(my_future).poll(cx) {
                        Poll::Ready(out) => {
                            assert_eq!(out, "done");
                            *self = Terminated;
                            return  Poll::Ready(());
                        }
                        Poll::Pending => {
                            return Poll::Pending;
                        }
                    }
                }
                Terminated => {
                    panic!("future polled after completion")
                }      
            }
        }
    }
}

// type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

struct MiniTokio {
    // tasks: VecDeque<Task>,    
    scheduled: mpsc::Receiver<Arc<Task>>,
    sender: mpsc::Sender<Arc<Task>>,
}

struct Task {
    // The `Mutex`is to make `Task` implement `Sync`. 
    // Only one thread accesses `future` at a given time.
    // The `Mutex` is not required for correctness.
    // Real Tokio does not use a mutex here, but real Tokio has more lines of code that can fit in a single tutorial page.

    future : Mutex<Pin<Box<dyn Future<Output =  ()> + Send>>>,
    executor: mpsc::Sender<Arc<Task>>,
}
impl Task {
    fn schedule (self: &Arc<Self>){
        self.executor.send(self.clone());
    }

    fn poll(self: Arc<Self>) {
        // Crate a waker from the `Task` instance.
        // This use the `ArcWake` impl from above.
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);

        // No other thread ever tries to lock the future
        let mut future = self.future.try_lock().unwrap();

        // Poll the future
        let _ = future.as_mut().poll(&mut cx);
    }

    // Spawn a new task with the given future.
    //
    // Initialize a new Task harness containing the given future and pushes it onto `sender`. The reciver half of the channel will get the task and execute it.

    fn spawn<F>(future: F, sender: &mpsc::Sender<Arc<Task>>) 
    where
        F: Future<Output = ()> + Send + 'static
    {
        let task = Arc::new(Task {
            future : Mutex::new(Box::pin(future)),
            executor: sender.clone(),
        });

        let _ = sender.send(task);
    }
    
}
impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.schedule();
    }
}
impl MiniTokio {
    // Intialize a new mini-tokio instance.
    fn new() -> MiniTokio {
        let (sender, scheduled) = mpsc::channel();
        MiniTokio {
            scheduled,
            sender
        }
    }

    // Spawn a future onto the mini-tokio instance..
    //
    // The given future is wrapped with the `Task` harness and pushed into the `sheduled` queue. the future will be executed when `run` is called.
    fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task::spawn(future, &self.sender);
    }

    fn run(&self) {
        while let Ok(task) = self.scheduled.recv() {
            task.poll();
        }
    }
}



#[tokio::main]
async fn main() {
    // let what_is_this = my_async_fn();

    // what_is_this.await;

    let when = Instant::now() + Duration::from_millis(10);
    let future = Delay { when };

    let out = future.await;
    assert_eq!(out, "done");
}


