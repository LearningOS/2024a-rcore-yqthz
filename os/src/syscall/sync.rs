use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
// use crate::syscall::sys_gettid;
// use crate::syscall::process;
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;

// use crate::task::MAX_THREADS;
// use crate::task::MAX_RESOURCES;


/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    
    let id = if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    };

    if process_inner.enable_dead_lock {
        println!("update available tid:{}  resource:{}", id as usize, 1);
        process_inner.update_available(id as usize, 1);          // add here
        println!("debug available: {:?}", process_inner.available);
        // drop(process_inner);
    }

    id

}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());

    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;

    if process_inner.enable_dead_lock {
        let lock = mutex.is_lock();
        if lock {
            // update need
            println!("update need tid:{}  resource_id:{}", tid, mutex_id);
            process_inner.update_need(tid, mutex_id);
            println!("debug need: {:?}", process_inner.need);
        }
        else {
            // update allocation
            println!("update allocation tid:{}  resource_id:{}", tid, mutex_id);
            process_inner.update_allocation(tid, mutex_id);
            println!("debug available: {:?}", process_inner.available);
            println!("debug allocation: {:?}", process_inner.allocation);
        }

        // 检测死锁
        println!("try to detect deadlock");
        // if deadlock_detect(process_inner.available, process_inner.allocation, process_inner.need) {
        //     drop(process_inner);
        //     return -0xdead;
        // }
        if process_inner.deadlock_detect() {
            drop(process_inner);
            drop(process);
            return -0xdead;
        }
    }

    drop(process_inner);
    drop(process);

    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());

    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;

    if  process_inner.enable_dead_lock {
        println!("release resources tid:{}  resource_id:{}", tid, mutex_id);
        process_inner.release_resources(tid, mutex_id);
    }


    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };

    if process_inner.enable_dead_lock {
        println!("update available tid:{}  resource:{}", id, res_count);
        process_inner.update_available(id, res_count as i32);          // add here
        println!("debug available: {:?}", process_inner.available);
        // drop(process_inner);
    }


    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());

    // let tid = sys_gettid() as usize;
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;

    if  process_inner.enable_dead_lock {
        println!("release resources tid:{}  resource_id:{}", tid, sem_id);
        process_inner.release_resources(tid, sem_id);
    }

    drop(process_inner);
    sem.up();

    // println!("tid: {} drop process_inner success", tid);

    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();

    // let tid = sys_gettid() as usize;
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    // println!("try to acquire process_inner  tid: {}, sem_id: {}", tid, sem_id);

    let mut process_inner = process.inner_exclusive_access();
    let thread_n = process_inner.tasks.len();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());


    // let tid = current_task()
    //     .unwrap()
    //     .inner_exclusive_access()
    //     .res
    //     .as_ref()
    //     .unwrap()
    //     .tid;

    
    if process_inner.enable_dead_lock {
        let count = sem.inner.exclusive_access().count;
        if count <= 0 {
            // update need
            println!("update need tid:{}  resource_id:{}", tid, sem_id);
            process_inner.update_need(tid, sem_id);
            println!("debug need: {:?}", process_inner.need);
        }
        else {
            // update allocation
            println!("update allocation tid:{}  resource_id:{}", tid, sem_id);
            process_inner.update_allocation(tid, sem_id);
            println!("debug available: {:?}", process_inner.available);
            println!("debug allocation: {:?}", process_inner.allocation);
        }

        if thread_n >= 2 {
            // 检测死锁
            println!("try to detect deadlock");
            // if deadlock_detect(process_inner.available, process_inner.allocation, process_inner.need) {
            //     drop(process_inner);
            //     return -0xdead;
            // }
            if process_inner.deadlock_detect() {
                drop(process_inner);
                return -0xdead;
            }
        }
    }

    drop(process_inner);
    sem.down();

    // println!("tid: {} drop process_inner success", tid);
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}

// pub fn deadlock_detect(available: [i32; MAX_RESOURCES], allocation: [[i32; MAX_RESOURCES]; MAX_THREADS], need: [[i32; MAX_RESOURCES]; MAX_THREADS]) -> bool {
//     let mut work = [0; MAX_RESOURCES];
//     for i in 0..MAX_RESOURCES {
//         work[i] = available[i];
//     }

//     let mut finish: [bool; MAX_THREADS] = [false; MAX_THREADS];

//     let mut found_thread = true;
    
//     println!("debug work: {:?}", work);

//     println!("debug need: {:?}", need);

//     while found_thread {
//         found_thread = false;

//         for i in 0..MAX_THREADS {
//             if !finish[i] {
//                 let mut can_allocate = true;
//                 for j in 0..MAX_RESOURCES {
//                     if need[i][j] > work[j] {
//                         can_allocate = false;
//                         break;
//                     }
//                 }

//                 if can_allocate {
//                     for j in 0..MAX_RESOURCES {
//                         work[j] += allocation[i][j];
//                     }
//                     finish[i] = true;
//                     found_thread = true;
//                 }
//             }
//         }
//     }

//     println!("debug finish: {:?}", finish);
//     for i in 0..MAX_THREADS {
//         if !finish[i] {
//             println!("detect deadlock");
//             return true;
//         }
//     }
//     println!("no detect deadlock");
//     false
// }


/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    if _enabled == 1 {
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.enable_dead_lock = true;
        drop(process_inner);
    }
    0
}
