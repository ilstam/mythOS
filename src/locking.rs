use crate::irq;
use aarch64_cpu::registers::DAIF;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};
use tock_registers::interfaces::{Readable, Writeable};

/// Spinlock that is never used from interrupt context
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

/// Spinlock that can be used from interrupt context
pub struct IRQSpinLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: Same as for SpinLock
unsafe impl<T: Send> Sync for IRQSpinLock<T> {}

#[allow(dead_code)]
impl<T> IRQSpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> IRQLockGuard<T> {
        let mut daif = DAIF.get();
        irq::disable_interrupts();

        while self.lock.swap(true, Ordering::Acquire) {
            DAIF.set(daif);

            while self.lock.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }

            daif = DAIF.get();
            irq::disable_interrupts();
        }

        IRQLockGuard {
            lock: self,
            old_daif: daif,
        }
    }
}

// The lifetime annotation means that the IRQLockGuard can't outlive the spinlock
pub struct IRQLockGuard<'a, T> {
    lock: &'a IRQSpinLock<T>,
    // The value of DAIF before the IRQLockGuard instance was created
    old_daif: u64,
}

impl<T> core::ops::Deref for IRQLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: The only way to create a IRQLockGuard instance is by calling
        // IRQSpinLock::lock(), hence exclusive access is guaranteed here
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for IRQLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: The only way to create a IRQLockGuard instance is by calling
        // IRQSpinLock::lock(), hence exclusive access is guaranteed here
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for IRQLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.store(false, Ordering::Release);
        DAIF.set(self.old_daif);
    }
}
