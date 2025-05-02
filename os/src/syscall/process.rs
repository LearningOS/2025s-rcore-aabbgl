//! Process management syscalls
use alloc::sync::Arc;
use crate::task::{
    TaskControlBlock,
    exit_current_and_run_next,
    suspend_current_and_run_next,
    current_task,
    current_user_token,
    get_current_task_page_table,
    create_new_map_area,
    unmap_consecutive_area,
    add_task,
};
use crate::loader::get_app_data_by_name;
use crate::config::{ PAGE_SIZE,  MAXVA};
use crate::timer::get_time_us;
use crate::mm::{translated_byte_buffer, translated_str, translated_refmut,VirtAddr, MapPermission, VPNRange};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    // 空指针检查
    if ts.is_null() {
        return -1;
    }

    // 获取当前用户态页表token
    let token = current_user_token();
    let time_val = TimeVal {
        sec: get_time_us() / 1_000_000,
        usec: get_time_us() % 1_000_000,
    };

    // 将TimeVal转换为字节序列
    let src = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const _ as *const u8,
            core::mem::size_of::<TimeVal>()
        )
    };

    // 获取用户空间缓冲区描述（自动处理跨页）
    let dst_buffers = translated_byte_buffer(
        token,
        ts as *const u8,
        core::mem::size_of::<TimeVal>()
    );

    // 检查缓冲区总长度是否足够
    let total_len: usize = dst_buffers.iter().map(|b| b.len()).sum();
    if total_len < src.len() {
        return -1;
    }

    // 分段拷贝数据
    let mut copied = 0;
    for buffer in dst_buffers {
        let len = buffer.len().min(src.len() - copied);
        buffer[..len].copy_from_slice(&src[copied..copied + len]);
        copied += len;
        if copied >= src.len() {
            break;
        }
    }

    0
}


/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    if start % PAGE_SIZE != 0 /* start need to be page aligned */ ||
        port & !0x7 != 0 /* other bits of port needs to be zero */ ||
        port & 0x7 ==0 /* No permission set, meaningless */ ||
        start >= MAXVA /* mapping range should be an legal address */ {
        return -1;
    }

    // check the range [start, start + len)
    let start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(start + len).ceil();
    let vpns = VPNRange::new(start_vpn, end_vpn);
    for vpn in vpns {
        if let Some(pte) = get_current_task_page_table(vpn) {
            // we find a pte that has been mapped
            if pte.is_valid() {
                return -1;
            }
        }
    }
    // all ptes in range has pass the test
    create_new_map_area(
        start_vpn.into(),
        end_vpn.into(),
        MapPermission::from_bits_truncate((port << 1) as u8) | MapPermission::U
    );
    0
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
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);

    // 获取当前任务和用户token
    let current_task = current_task().unwrap();
    let token = current_user_token();

    // 解析用户空间传入的路径
    let path = translated_str(token, path);

    // 获取ELF文件数据
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        // 使用Arc包装新创建的任务
        let new_task = Arc::new(TaskControlBlock::new(data));
        let new_pid = new_task.getpid();

        // 设置父子进程关系时使用Arc的克隆
        {
            let mut parent_inner = current_task.inner_exclusive_access();
            let mut child_inner = new_task.inner_exclusive_access();
            child_inner.parent = Some(Arc::downgrade(&current_task));
            parent_inner.children.push(Arc::clone(&new_task)); // 改为Arc克隆
        }

        // 加入调度队列
        add_task(new_task.into());
        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    // 参数校验：有效范围是 prio >= 2
    if prio < 2 {
        return -1;
    }

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.priority = prio as u64;

    trace!(
        "kernel:pid[{}] set priority to {}",
        task.pid.0,
        prio
    );
    prio // 成功时返回设置的优先级值
}
