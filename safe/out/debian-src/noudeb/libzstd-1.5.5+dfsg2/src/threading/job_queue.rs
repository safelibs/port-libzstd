use std::{
    fmt,
    panic::{self, AssertUnwindSafe},
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

type Task = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Run(Task),
    Stop,
}

#[derive(Debug)]
pub(crate) enum JobError {
    QueueClosed,
    WorkerPanicked,
}

pub(crate) struct JobHandle<T> {
    state: Arc<JobResult<T>>,
}

struct JobResult<T> {
    result: Mutex<Option<Result<T, JobError>>>,
    ready: Condvar,
}

impl<T> JobResult<T> {
    fn new() -> Self {
        Self {
            result: Mutex::new(None),
            ready: Condvar::new(),
        }
    }

    fn store(&self, result: Result<T, JobError>) {
        let mut guard = self
            .result
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = Some(result);
        self.ready.notify_one();
    }
}

impl<T> JobHandle<T> {
    pub(crate) fn try_wait(&mut self) -> Option<Result<T, JobError>> {
        let mut guard = self
            .state
            .result
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.take()
    }

    pub(crate) fn wait(self) -> Result<T, JobError> {
        let mut guard = self
            .state
            .result
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        loop {
            if let Some(result) = guard.take() {
                return result;
            }
            guard = self
                .state
                .ready
                .wait(guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

pub(crate) struct JobQueue {
    sender: mpsc::Sender<Message>,
    workers: Vec<JoinHandle<()>>,
}

impl fmt::Debug for JobQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JobQueue")
            .field("workers", &self.workers.len())
            .finish_non_exhaustive()
    }
}

impl JobQueue {
    pub(crate) fn new(worker_count: usize) -> Self {
        let worker_count = worker_count.max(1);
        let (sender, receiver) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let receiver = Arc::clone(&receiver);
            workers.push(thread::spawn(move || loop {
                let message = {
                    let receiver = match receiver.lock() {
                        Ok(receiver) => receiver,
                        Err(_) => break,
                    };
                    receiver.recv()
                };

                match message {
                    Ok(Message::Run(task)) => task(),
                    Ok(Message::Stop) | Err(_) => break,
                }
            }));
        }

        Self { sender, workers }
    }

    pub(crate) fn submit<F, T>(&self, job: F) -> Result<JobHandle<T>, JobError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let state = Arc::new(JobResult::new());
        let task_state = Arc::clone(&state);
        let task = Box::new(move || {
            let result =
                panic::catch_unwind(AssertUnwindSafe(job)).map_err(|_| JobError::WorkerPanicked);
            task_state.store(result);
        });

        self.sender
            .send(Message::Run(task))
            .map_err(|_| JobError::QueueClosed)?;
        Ok(JobHandle { state })
    }
}

impl Drop for JobQueue {
    fn drop(&mut self) {
        for _ in 0..self.workers.len() {
            let _ = self.sender.send(Message::Stop);
        }

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JobQueue;
    use std::sync::{Arc, Barrier};

    #[test]
    fn jobs_run_concurrently_on_worker_threads() {
        let queue = JobQueue::new(2);
        let barrier = Arc::new(Barrier::new(3));
        let first_barrier = Arc::clone(&barrier);
        let second_barrier = Arc::clone(&barrier);

        let first = queue
            .submit(move || {
                first_barrier.wait();
                std::thread::current().id()
            })
            .expect("submit first job");
        let second = queue
            .submit(move || {
                second_barrier.wait();
                std::thread::current().id()
            })
            .expect("submit second job");

        barrier.wait();
        let first_thread = first.wait().expect("first job result");
        let second_thread = second.wait().expect("second job result");

        assert_ne!(first_thread, second_thread);
        assert_ne!(first_thread, std::thread::current().id());
        assert_ne!(second_thread, std::thread::current().id());
    }
}
