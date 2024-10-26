//! Process management syscalls
use core::mem::size_of;

// use alloc::task;

// use riscv::addr::Frame;


use crate::{ 
    config::{MAX_SYSCALL_NUM, PAGE_SIZE}, mm::{frame_alloc, translated_byte_buffer, MapPermission, VirtAddr}, task::{
        change_program_brk, current_user_token, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus
    }, timer::{get_time_ms, get_time_us}
};
use crate::task::TASK_MANAGER;
use crate::mm::{MapArea, MapType, PTEFlags};

#[repr(C)]
#[derive(Debug)]
/// time_val
pub struct TimeVal {
    /// s
    pub sec: usize,
    /// us
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}

impl TaskInfo {
    /// init TaskInfo
    pub fn new() -> Self {
        Self {
            status: TaskStatus::Ready,
            syscall_times: [0u32; MAX_SYSCALL_NUM],
            time: 0,
        }
    }
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let user_token = current_user_token();
    let sec = get_time_ms() / 1000;
    let usec = get_time_us();
    let mut time_val = translated_byte_buffer(user_token, _ts as *const u8,  size_of::<TimeVal>());
    let sec_bytes = sec.to_le_bytes();
    let usec_bytes = usec.to_le_bytes();
    // time_val在一个page中
    if time_val.len() == 1 {
        for i in 0..sec_bytes.len() {
            time_val[0][i] = sec_bytes[i];
        } 
        for i in 0..usec_bytes.len() {
            time_val[0][i + 8] = usec_bytes[i];
        } 
    }
    else {
        // time_val不在一个page中
        let length = time_val[0].len();
        for i in 0..length {
            time_val[0][i] = sec_bytes[i];
        }
        if length < 8 {
            for i in 0..8 - length {
                time_val[1][i] = sec_bytes[i + length];
            }
        }
        for i in 0..usec_bytes.len() {
            time_val[1][i + 8 - length] = usec_bytes[i];
        }

    }
 
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let user_token = current_user_token();
    let mut task_info = translated_byte_buffer(user_token, _ti as *const u8,  size_of::<TaskInfo>());

    let inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;

    let status = inner.tasks[current].task_info.status;
    let syscall_times = inner.tasks[current].task_info.syscall_times;
    let old_time = inner.tasks[current].task_info.time;
    let time = get_time_ms() - old_time;
    drop(inner);

    // task_info在一个page中
    if task_info.len() == 1 {
        let mut count = 0;
        // copy status
        // let mut status_bytes = [0u8; size_of::<TaskStatus>()];
        // status_bytes[0] = status as u8;
        // task_info[0][count] = status_bytes[0];
        // count += 8;
        // copy sys_times
        for i in 0..MAX_SYSCALL_NUM {
            // assert_eq!(count, i * 4 + 8);
            if syscall_times[i] == 0 {
                count += 4;
                continue;
            }
            let byte = syscall_times[i].to_le_bytes();
            for j in 0..byte.len() {
                task_info[0][count] = byte[j];
                count += 1;
            }
        }


        // copy time
        let time_byte = time.to_le_bytes();
        for i in 0..time_byte.len() {
            task_info[0][count] = time_byte[i];
            count += 1;
        }

        // copy status
        let mut status_bytes = [0u8; size_of::<TaskStatus>()];
        status_bytes[0] = status as u8;
        task_info[0][count] = status_bytes[0];
        // count += 8;

    }
    else {
        // task_info不在一个page中
        let mut count = 0;
        // copy status
        // let mut status_bytes = [0u8; size_of::<TaskStatus>()];
        // status_bytes[0] = status as u8;
        // task_info[0][count] = status_bytes[0];

        let length = task_info[0].len();

        // copy sys_times
        let mut k = 0;
        for i in 0..MAX_SYSCALL_NUM {
            if syscall_times[i] == 0 {
                count += 4;
                if count >= length {
                    let n = count - length;
                    count = n;
                    k += 1;
                }
                continue;
            }
            let byte = syscall_times[i].to_le_bytes();
            
            for j in 0..byte.len() {
                task_info[k][count] = byte[j];
                count += 1;
                if count == length {
                    count = 0;
                    k += 1;
                }
            }
        }

        // copy time
        let time_byte = time.to_le_bytes();
        for i in 0..time_byte.len() {
            task_info[0][count] = time_byte[i];
            count += 1;
            if count == length {
                count = 0;
                k += 1;
            }
        }

        // copy status
        let mut status_bytes = [0u8; size_of::<TaskStatus>()];
        status_bytes[0] = status as u8;
        task_info[k][count] = status_bytes[0];
    }
    0
}


// YOUR JOB: Implement mmap.
/// mmap
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    println!("try to map {} to {}", _start, _start + _len);
    let start_va = VirtAddr::from(_start);
    if start_va.page_offset() != 0 {
        // start 没有按页对齐
        println!("start should align page");
        return -1;
    }
    if _port & !0x7 != 0 {
        // 其他位必须为0
        println!("other bit should be zero");
        return -1;
    }
    if _port & 0x7 == 0 {
        // 没有意义
        println!("no sense");
        return -1;
    }

    let mut permission = MapPermission::U;
    let mut pte_flag = PTEFlags::V | PTEFlags::U;
    if _port & 0x1 != 0 {
        permission |= MapPermission::R;
        pte_flag |= PTEFlags::R;
    }
    if _port & 0x2 != 0 {
        permission |= MapPermission::W;
        pte_flag |= PTEFlags::W;
    }
    if _port & 0x4 != 0 {
        permission |= MapPermission::X;
        pte_flag |= PTEFlags::U;
    }

    let end = _start + _len;
    // let end_va = VirtAddr::from(end);
    let mut start = _start;

    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let memory_set = &mut inner.tasks[current].memory_set;
    let page_table = &mut memory_set.page_table;

    while start < end {
        let start_va1 = VirtAddr::from(start); 
        let end_va1 = VirtAddr::from(start_va1.0 + PAGE_SIZE);
        let mut map_area = MapArea::new(start_va1, end_va1, MapType::Framed, permission);


        for vpn in map_area.vpn_range {
            // 检测是否映射过
            println!("vpn: {:?}", vpn);
            let pte = page_table.find_pte(vpn);
            if pte.is_some() && pte.unwrap().is_valid() {
                // 已经映射过
                println!("page is mapped before");
                return -1;
            }
            // 分配一个page
            let frame = frame_alloc();
            if frame.is_none() {
                // 物理内存不足
                println!("no enought physical memory");
                return -1;
            }
            let framed = frame.unwrap();
            let ppn = framed.ppn;
            map_area.data_frames.insert(vpn, framed);
            // let pte_flags = PTEFlags::from_bits(map_area.map_perm.bits).unwrap();
            page_table.map(vpn, ppn, pte_flag);
        }
        memory_set.areas.push(map_area);
        
        // memory_set.push(map_area, None);
        start = end_va1.into();
    }
    drop(inner);
    println!("map {} to {} succeed", _start, _start + _len);
    0
}

// YOUR JOB: Implement munmap.
/// munmap
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    println!("try to munmap {} to {}", _start, _start + _len);

    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let memory_set = &mut inner.tasks[current].memory_set;
    let page_table = &mut memory_set.page_table;

    let mut start = _start;
    let end = _start + _len;

    while start < end {
        let start_va1 = VirtAddr::from(start); 
        let end_va1 = VirtAddr::from(start_va1.0 + PAGE_SIZE);
        

        let mut is_find = false;
        let mut index = 0;
        for (i, area) in memory_set.areas.iter().enumerate() {
            for vpn in area.vpn_range {
               let va = VirtAddr::from(vpn).0;
               if va <= start_va1.0 && va + PAGE_SIZE >= end_va1.0 {
                    index = i;
                    is_find = true;
                    break;
               } 
            }
            if is_find {
                break;
            }
        }
        if !is_find {
            println!("can't find area");
            return -1;
        }

        let map_area = &mut memory_set.areas[index]; 

        for vpn in map_area.vpn_range {
            // 检测是否映射过
            let pte = page_table.find_pte(vpn);
            if pte.is_none() {
                // 没有映射过
                println!("page is not mapped before");
                return -1;
            }
            if map_area.map_type == MapType::Framed {
                map_area.data_frames.remove(&vpn);
            }
            page_table.unmap(vpn);
            // map_area.unmap_one(page_table, vpn);
        }
        
        // memory_set.areas.remove(index);
        start = end_va1.into();
    }
    drop(inner);
    println!("munmap {} to {} succeed", _start, _start + _len);
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
