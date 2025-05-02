
## stride算法
在 TaskControlBlockInner 中定义了两个关键字段：
priority：表示进程的优先级，数值越大优先级越低（即调度频率越低）
stride：表示当前步幅值，每次调度后会根据优先级更新
通过两个方法来实现stride调度：
Ord trait 实现了进程之间的比较逻辑，确保调度器可以按照 stride 值从小到大进行排序。
PartialEq 实现了相等性判断，仅用于调度队列中的比较

在 run_tasks() 函数中，当调度器选择一个任务运行时，会更新其 stride 值：
pass 表示本次调度应增加的步幅值，等于 BIG_STRIDE / priority。
高优先级任务（priority 较小）会获得较小的 pass，因此步幅增长较慢，更容易被再次调度。

## 简答题
1.p2 会被再次调度，而非 p1。 原因：
Stride 算法通过比较步幅值选择最小的进程运行。
在溢出情况下，255 和 4 的直接比较会认为 4 更小，因此 p2 会被选中。



2.通过限制进程优先级 ≥ 2，可以确保：
pass = BIG_STRIDE / priority 的最大值为 BIG_STRIDE / 2。
任何两个进程的步幅差值不会超过 BIG_STRIDE / 2。

3.
```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // 将 u64 步幅值转换为 i64 并计算差值
        let diff = (self.0 as i64).wrapping_sub(other.0 as i64);
        if diff < 0 {
            Some(Ordering::Less)     // self < other
        } else {
            Some(Ordering::Greater)  // self > other
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false  // 假设两个 Stride 永远不会相等
    }
}

```

## 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
    
  
    
2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
    
    > https://blog.csdn.net/mzz510/article/details/136109343
    

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。