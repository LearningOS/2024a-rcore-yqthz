1. 实现sys_spawn系统调用和stride调度算法
2.  

    1. 
      - 不是, p2的 stride 累加其 pass 值10后溢出到4
      - 如果所有进程的步长都较小（即优先级接近且较高），累加后 stride 值的变化幅度很小，此时不同进程的 stride 值就非常接近。这种情况下，stride 值在接近 BigStride（255）时容易溢出，使得两者在溢出后差异变大，导致选错进程
        如果所有进程的优先级满足优先级>=2, 那么所有 pass 值满足passi <= BigStride / 2, 即每次更新后 stride 值差异也不会超过 BigStride / 2

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let big_stride = 255;
        let diff = self.0.wrapping_sub(other.0);
        if diff <= big_stride / 2 {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。
4.  我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计

