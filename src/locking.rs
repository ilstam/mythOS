use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct SpinLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: The compiler is not happy because UnsafeCell is not thread-safe.
// However, when we wrap it in our SpinLock implementation it is safe to
// share between threads because we can only access its data after aquiring
// the lock which we do using atomic operations.
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> LockGuard<T> {
        // We use two loops here to reduce cache coherence traffic. The swap()
        // is a write operation that will move the cache line to a 'modified'
        // or 'exclusive' state, whereas load() is a read operation and
        // multiple CPUs can have the cache line in the 'shared' state when
        // there is contention for the lock.
        while self.lock.swap(true, Ordering::Acquire) {
            while self.lock.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }

        LockGuard { lock: self }
    }
}

// The lifetime annotation means that the LockGuard can't outlive the spinlock
pub struct LockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> core::ops::Deref for LockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: The only way to create a LockGuard instance is by calling
        // SpinLock::lock(), hence exclusive access is guaranteed here
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for LockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: The only way to create a LockGuard instance is by calling
        // SpinLock::lock(), hence exclusive access is guaranteed here
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.store(false, Ordering::Release);
    }
}
