use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mimi_core::state_machine::{MimiState, StateManager, Task, TaskPriority, TaskType};
use std::time::Duration;

fn bench_state_transitions(c: &mut Criterion) {
    c.bench_function("state_machine_happy_path_transitions", |b| {
        b.iter(|| {
            let manager = StateManager::new();
            manager.transition_to(MimiState::Listening).unwrap();
            manager.transition_to(MimiState::Processing).unwrap();
            manager.transition_to(MimiState::Executing).unwrap();
            manager.transition_to(MimiState::Responding).unwrap();
            manager.transition_to(MimiState::Idle).unwrap();
            black_box(manager.current_state());
        });
    });
}

fn bench_task_queue_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_machine_task_queue");

    for batch in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(batch), batch, |b, &batch| {
            b.iter(|| {
                let manager = StateManager::with_capacity(batch);
                for i in 0..batch {
                    let mut task = Task::new(TaskType::Query, "bench-task");
                    task.priority = if i % 4 == 0 {
                        TaskPriority::Critical
                    } else if i % 3 == 0 {
                        TaskPriority::High
                    } else {
                        TaskPriority::Normal
                    };
                    task.timeout = Duration::from_millis(50);
                    manager.enqueue_task(task).unwrap();
                }

                let mut dequeued = 0usize;
                while manager.queue_size() > 0 {
                    manager.dequeue_task().unwrap();
                    dequeued += 1;
                }
                black_box(dequeued);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_state_transitions, bench_task_queue_throughput);
criterion_main!(benches);
