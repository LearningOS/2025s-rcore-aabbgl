//! Process management syscalls
use crate::task::{ unmap_consecutive_area, change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, current_user_token, TASK_MANAGER};
use crate::timer::get_time_us;
use crate::mm::{MapPermission, VirtAddr};
use crate::mm::page_table::{translated_refmut, translated_ref};
use crate::config::{PAGE_SIZE, MAXVA,MEMORY_END};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
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
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    if ts.is_null() {
        return -1;
    }
    let time_val = translated_refmut::<TimeVal>(current_user_token(), ts);
    if time_val.is_err() {
        return -1;
    }
    let time_val = time_val.unwrap();
    let us = get_time_us();
    time_val.sec = us / 1_000_000;
    time_val.usec = us % 1_000_000;
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");

    match trace_request {
        // 读取用户空间字节
        0 => {
            // 检查地址是否超过最大合法地址
            if id >= MEMORY_END{
                return -1;
            }

            let user_token = current_user_token();
            let user_data = translated_ref::<u8>(user_token, id as *const u8);

            if let Ok(data_ref) = user_data {
                *data_ref as isize
            } else {
                -1 // 地址无效或不可读
            }
        }

        // 写入用户空间字节
        1 => {
            // 检查地址是否超过最大合法地址
            if id >= MEMORY_END {
                return -1;
            }

            let user_token = current_user_token();
            let user_data = translated_refmut::<u8>(user_token, id as *mut u8);

            if let Ok(data_ref) = user_data {
                *data_ref = data as u8;
                0 // 返回成功
            } else {
                -1 // 地址无效或不可写
            }
        }

        // 查询系统调用次数（保持不变）
        2 => TASK_MANAGER.get_syscall_counts(id) as isize,

        // 无效请求
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    if start % PAGE_SIZE != 0 /* start need to be page aligned */ ||
        prot & !0x7 != 0 /* other bits of prot needs to be zero */ ||
        prot & 0x7 == 0 /* No permission set, meaningless */ ||
        start >= MAXVA /* mapping range should be an legal address */ {
        return -1;
    }


    // all ptes in range have passed the test
    let start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(start + len).ceil();
    let perm = MapPermission::from_bits_truncate((prot << 1) as u8) | MapPermission::U;

    TASK_MANAGER.mmap_current_task(start_vpn, end_vpn, perm)
}

/// munmap the mapped virtual addresses
pub fn sys_munmap(start: usize, len: usize) -> isize {
    if start >= MAXVA || start % PAGE_SIZE != 0 {
        return -1;
    }
    // avoid undefined situation
    let mut mlen = len;
    if start > MAXVA - len {
        mlen = MAXVA - start;
    }
    unmap_consecutive_area(start, mlen)
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
