//! File and filesystem-related syscalls

use crate::fs::{open_file, OSInode, OpenFlags, Stat, create_file_link, unlink_file};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
use core::mem::size_of;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        println!("open a file {}", fd);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat",
        current_task().unwrap().pid.0
    );
    println!("acquire file {} stat", _fd);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if _fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[_fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);

        let osinode = file.as_any().downcast_ref::<OSInode>();
        if osinode.is_none() {
            return -1;
        } 

        let inner = osinode.unwrap().inner.exclusive_access();
        let ino = inner.inode.block_id;
        let is_file = inner.inode.read_disk_inode(|inode| inode.is_file());
        let nlink = inner.inode.read_disk_inode(|disk_inode| disk_inode.get_nlink());
        println!("file {} nlink: {}", _fd, nlink);

        let mut file_stat = translated_byte_buffer(token, _st as *const u8,  size_of::<Stat>());

        let ino_bytes = ino.to_le_bytes();
        let bytes;
        if is_file {
            let f :u32 = 0o100000;
            bytes = f.to_le_bytes();
        }
        else {
            let f: u32 = 0o040000;
            bytes = f.to_le_bytes();
        }
        let nlink_bytes = nlink.to_le_bytes();

        // file_stat 在一个page中
        if file_stat.len() == 1 {
            let mut count = 0;
            // copy dev
            count += 8;
            // copy ino
            for i in 0..ino_bytes.len() {
                file_stat[0][count] = ino_bytes[i];
                count += 1;
            }
            // copy mod
            for i in 0..bytes.len() {
                file_stat[0][count] = bytes[i];
                count += 1;
            }
            // copy nlink
            for i in 0..nlink_bytes.len() {
                file_stat[0][count] = nlink_bytes[i];
                count += 1;
            }

        }
        else {
            let mut count = 0;
            let mut k = 0;
            let length = file_stat[0].len();
            if length < 8 {
                let n = 8 - length;
                count += n;
                k += 1;
            }
            else {
                count += 8;
            }
            // copy ino
            for i in 0..ino_bytes.len() {
                file_stat[k][count] = ino_bytes[i];
                count += 1;
                if count == length {
                    count = 0;
                    k += 1;
                }
            }
            // copy mod
            for i in 0..bytes.len() {
                file_stat[k][count] = bytes[i];
                count += 1;
                if count == length {
                    count = 0;
                    k += 1;
                }
            }
            // copy nlink
            for i in 0..nlink_bytes.len() {
                file_stat[k][count] = nlink_bytes[i];
                count += 1;
                if count == length {
                    count = 0;
                    k += 1;
                }
            }
        }
    } 
    else {
        return -1
    }
    0
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let old_name = translated_str(token, _old_name);
    let new_name = translated_str(token, _new_name);
    if old_name == new_name {
        return -1;
    }
    if let Some(file) = open_file(&old_name, OpenFlags::RDONLY) {
        let inner = file.inner.exclusive_access();
        inner.inode.increase_nlink();
        create_file_link(&old_name, &new_name);

        // println!("file old_name {}, new_name {} nlink: {}", old_name, new_name, inner.inode.nlink);
    } else {
        return -1;
    }
    0
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let name = translated_str(token, _name);
    if let Some(inode) = open_file(&name, OpenFlags::RDONLY) {
        let inner = inode.inner.exclusive_access();
        inner.inode.decrease_nlink();
        unlink_file(&name);
    } else {
        return -1;
    }
    0
}
