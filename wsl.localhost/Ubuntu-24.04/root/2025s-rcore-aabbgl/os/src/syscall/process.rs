
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
