
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");

    // 检查 start 是否按页对齐
    if start % PAGE_SIZE != 0 {
        return -1;
    }

    // 检查 prot 是否合法
    if (prot & !0x7) != 0 || (prot & 0x7) == 0 {
        return -1;
    }

    // 计算需要映射的页数（向上取整）
    let len_in_pages = (len + PAGE_SIZE - 1) / PAGE_SIZE;

    // 获取当前任务的内存集
    let token = current_user_token();
    let mut memory_set = MemorySet::from_token(token);

    // 转换起始虚拟地址为虚拟页号
    let start_vpn = VirtAddr::from(start).floor();

    // 检查目标虚存区间是否已经被映射
    for i in 0..len_in_pages {
        let vpn = start_vpn + i;
        if memory_set.translate(vpn).is_some() {
            warn!("Target virtual memory range is already mapped: {:?}", vpn);
            return -1; // 目标地址已被映射，返回错误
        }
    }

    // 根据 prot 设置页表项的权限
    let flags = match prot {
        1 => MapPermission::R,
        2 => MapPermission::W,
        3 => MapPermission::R | MapPermission::W,
        4 => MapPermission::X,
        5 => MapPermission::R | MapPermission::X,
        6 => MapPermission::W | MapPermission::X,
        7 => MapPermission::R | MapPermission::W | MapPermission::X,
        _ => return -1, // 不可能到达这里，因为前面已经检查过 prot
    };

    // 映射虚存页到物理页
    for i in 0..len_in_pages {
        let vpn = start_vpn + i;
        let ppn = memory_set.alloc_frame(); // 分配物理页

        if ppn.is_none() {
            error!("Failed to allocate physical frame for virtual page: {:?}", vpn);
            return -1; // 物理内存不足
        }

        let start_vpn = VirtAddr::from(vpn);
        let end_vpn = VirtAddr::from(vpn + 1);

        memory_set.insert_framed_area(
            start_vpn,
            end_vpn,
            flags | MapPermission::U,
        );
    }

    // 更新当前任务的内存集
    TASK_MANAGER.change_current_memory_set(memory_set);

    0 // 返回成功
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");

    // 检查 start 是否按页对齐
    if start % PAGE_SIZE != 0 {
        return -1;
    }

    // 计算需要解映射的页数（向上取整）
    let len_in_pages = (len + PAGE_SIZE - 1) / PAGE_SIZE;

    // 获取当前任务的内存集
    let token = current_user_token();
    let mut memory_set = MemorySet::from_token(token);

    // 转换起始虚拟地址为虚拟页号
    let start_vpn = VirtAddr::from(start).floor();

    // 检查目标虚存区间是否已经被映射
    let mut all_unmapped = true;
    for i in 0..len_in_pages {
        let vpn = start_vpn + i;
        if memory_set.translate(vpn).is_some() {
            all_unmapped = false;
            break;
        }
    }

    // 如果所有目标页都未被映射，则直接返回成功
    if all_unmapped {
        return 0;
    }

    // 解映射虚存页
    for i in 0..len_in_pages {
        let vpn = start_vpn + i;
        if memory_set.translate(vpn).is_some() {
            memory_set.unmap(vpn);
        }
    }

    // 更新当前任务的内存集
    TASK_MANAGER.change_current_memory_set(memory_set);

    0 // 返回成功
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    if ts.is_null() {
        return -1;
    }
    let user_token = current_user_token();
    let time_val = translated_refmut::<TimeVal>(user_token, ts);

    // 检查用户空间指针是否有效
    if time_val.is_err() {
        error!("Invalid user space pointer in sys_get_time");
        return -1;
    }

    let time_val = time_val.unwrap();
    let us = get_time_us();

    // 确保时间单位转换逻辑正确
    let sec = us / 1_000_000;
    let usec = us % 1_000_000;

    // 打印调试信息
    debug!("sys_get_time: sec = {}, usec = {}", sec, usec);

    // 写入用户空间
    time_val.sec = sec;
    time_val.usec = usec;

    0
}

pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");

    match trace_request {
        // 读取用户空间的一个字节
        0 => {
            let user_token = current_user_token();
            let user_data = translated_ref::<u8>(user_token, id as *const u8);

            if user_data.is_err() {
                return -1; // 地址无效或不可读
            }

            *user_data.unwrap() as isize
        }

        // 写入用户空间的一个字节
        1 => {
            let user_token = current_user_token();
            let user_data = translated_refmut::<u8>(user_token, id as *mut u8);

            if user_data.is_err() {
                return -1; // 地址无效或不可写
            }

            *user_data.unwrap() = data as u8;
            0 // 返回成功
        }

        // 查询系统调用次数
        2 => {
            TASK_MANAGER.get_syscall_counts(id) as isize
        }

        // 非法请求
        _ => -1,
    }
}
