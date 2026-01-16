# sink slow/stop 丢数问题排查备忘

## 背景
- 案例：`wp-examples/core/slower_sink` 的 `slow_bkh` sink 配置 `sleep_ms=100`，每 100ms 处理一条记录。
- batch 模式下 1 万条数据会在 ~2s 内被 source/pick/parse 完成，此时 control plane 立即下发 `Stop(Immediate)`。
- sink 线程 `work-sink:demo` 仍在消费 backlog，但在收到 `CtrlCmd::Stop(StopNow)` 时立刻退出，统计仅剩 3840 条成功，其余 6160 丢失。

## 根因
1. `SinkDispatcher` 使用 `tokio::sync::mpsc` 通道（cap=128）缓存待下发记录。虽然慢 sink 导致队列积压，但数据仍留在 channel 中。
2. `SinkWork::async_proc`（`src/runtime/sink/act_sink.rs:48-85`）在 `tokio::select!` 中监听「数据包」「控制命令」「修复句柄」。当 `TaskController` 收到 Stop，即刻 `break` 退出循环。
3. 退出后马上调用 `sink.proc_end()` → `sink_rt.primary.stop()`，没有 drain `SinkDatYReceiver` 中剩余的包，通道里的记录被丢弃。
4. batch 模式因为上游已经跑完，Stop 会很快抵达；daemon 模式在停机时也会触发相同逻辑，因此理论上也会丢弃停机瞬间仍在队列中的数据。

## 影响
- 只要 sink 的速率低于 parse，batch 任务必然出现“统计成功 < 输入数”的情况。
- daemon 模式下如果某条控制指令要求停机，也会因为缺失 drain 而把管道里剩余的数据 drop，对多 sink 组都适用（包括 infra sinks）。

## 建议方案
1. **两阶段关停**：Stop 到达后不要立刻 break，而是切换为 drain 状态，继续从 channel 中 `recv()` 直到 sender 全部关闭。
2. **待处理计数**：利用 `group_sink_package` 返回的处理条数来追踪 backlog；只有当 `recv()` 返回 `None` 且 `pending == 0` 时才真正退出。
3. **超时/防阻塞**：为 daemon 模式增加可配置的 drain timeout（如默认 60s），超过则输出告警后强制退出，防止异常 sink 卡死停机流程。
4. **Infra sink 同步改造**：`async_proc_infra` 也需要相同 drain 判定，避免 default/miss/residue/monitor sink 丢数。
5. **可观测性**：drain 阶段记录日志（`start sink drain` / `pending_left=xxx`），必要时通过 monitor sink 或 metrics 报告等待时长。

## run_mode 依赖？
- 当前 sink 工作线程并不知道 RunMode；但“先 drain 再退出”是通用刚需，与 batch/daemon 无关。
- 因此无需向下传递新的 batch 标志。实现上只需在 sink worker 层面统一处理，daemon 多加一个超时即可。
- Batch 模式会立即受益：慢 sink 也能在 stop 后自然耗尽 backlog；Daemon 模式在维护停机时也能保证“先处理完手头数据再下线”。

## 实施与验证
- 代码：`src/runtime/sink/act_sink.rs` 现已在 Stop 阶段调用 `close_channel()`，确保 drain 仅在 channel 真正关闭后退出，并在 `DrainState` 上集中处理 Pending/Drained 判定。
- 测试：为 `DrainState` 新增单元测试（`src/runtime/sink/drain.rs`），覆盖“未进入 drain 直接关闭”“drain 后关闭”“receiver::close 解锁”的场景；另外通过 `wp-examples/core/slower_sink/run.sh` 实测 slow sink (`sleep_ms=100`) 成功在 10k backlog drain 完成后退场。
- 文档：本文档同步更新，明确 batch/daemon 均采取“Stop→drain→shutdown”，并提醒运营在慢 sink 时需要根据 backlog 预估额外时间。

## 后续考虑
- 若 daemon 场景担心 drain 过久，可在 `DrainState` 上附加可配置超时，超时后记录警告并强制退出。
- 可观测性可进一步纳入 monitor sink/metrics：统计 drain 耗时、剩余 backlog，帮助运维判断是否需要调整 sink 并发或 `sleep_ms`。
