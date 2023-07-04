use std::sync::{RwLock, Mutex, Condvar};

/// Possible errors where using a barrier.
#[derive(Debug)]
pub enum BarrierError {
    /// Barrier is closed.
    Closed,
}

/// A light wrapper around a mutex and condition variable.
/// It is used for inter-thread communication for workload synchronisation.
/// 
/// # Examples
/// ```
/// use std::sync::Arc;
/// use ofdm_demod::barrier::Barrier;
/// 
/// let barrier = Arc::new(Barrier::new(false));
/// 
/// let thread_0 = std::thread::spawn({
///     let barrier = barrier.clone();
///     move || {
///         barrier.wait(|state| *state).unwrap();
///         println!("[thread-0] passed barrier");
///         barrier.acquire().set(false).unwrap();
///         println!("[thread-0] updated barrier");
///     }
/// });
/// 
/// let thread_1 = std::thread::spawn({
///     let barrier = barrier.clone();
///     move || {
///         // Acquire lets use get a mutable reference through Arc<>
///         barrier.acquire().set(true).unwrap();
///         println!("[thread-1] updated barrier");
///         barrier.wait(|state| !*state).unwrap();
///         println!("[thread-1] passed barrier");
///     }
/// });
/// 
/// thread_0.join().unwrap();
/// thread_1.join().unwrap();
/// ```
pub struct Barrier<T> {
    data: Mutex<T>,
    is_closed: RwLock<bool>,
    on_change: Condvar,
}

#[allow(unused)]
impl<T> Barrier<T> {
    pub fn new(initial_data: T) -> Self {
        Self {
            data: Mutex::new(initial_data),
            is_closed: RwLock::new(false),
            on_change: Condvar::new(),
        }
    }

    /// Forcefully updates all threads waiting for an update.
    pub fn notify_all(&mut self) {
        self.on_change.notify_all();
    }

    /// Close the barrier.
    /// If there are threads waiting for or updating the barrier they will get a Closed error.
    pub fn close(&mut self) -> Result<(),BarrierError> {
        let mut is_closed = self.is_closed.write().unwrap();
        if *is_closed {
            return Err(BarrierError::Closed);
        }
        *is_closed = true;
        self.on_change.notify_all();
        Ok(())
    }

    /// Gets a mutable reference to the barrier through a mutable access.
    /// This code should be safe since we prevent data races with locks in all mutable methods.
    /// This allows for multiple owners to update the internal state through something like Arc.
    pub fn acquire<'a>(&'a self) -> &'a mut Self {
        unsafe { &mut *(self as *const Self as *mut Self) }
    }
}

#[allow(unused)]
impl<T> Barrier<T> where T: PartialEq {
    /// Blocks thread until the predicate has been satisfied.
    pub fn wait(&self, predicate: impl Fn(&T) -> bool) -> Result<(),BarrierError> {
        let mut data = self.data.lock().unwrap();
        loop {
            {
                if *self.is_closed.read().unwrap() {
                    return Err(BarrierError::Closed);
                }
            }
            if predicate(&*data) {
                break;
            }
            data = self.on_change.wait(data).unwrap();
        }
        Ok(())
    }

    /// Updates the barrier with a new value and notifies all threads waiting on the barrier.
    pub fn set(&mut self, new_data: T) -> Result<(),BarrierError> {
        if *self.is_closed.read().unwrap() {
            return Err(BarrierError::Closed);
        }

        let mut state = self.data.lock().unwrap();
        *state = new_data;
        self.on_change.notify_all();
        Ok(())
    }
}

impl<T> Drop for Barrier<T> {
    /// Close the barrier when it falls out of scope.
    fn drop(&mut self) {
        self.close().unwrap_or(());
    }
}
